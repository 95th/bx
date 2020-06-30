use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::convert::TryInto;
use std::fmt;
use std::hash::Hash;
use std::marker::PhantomData;

pub type Result<T> = std::result::Result<T, Error>;

pub enum Error {
    Eof,
    Type { reason: &'static str },
    Length { expected: usize, actual: usize },
    Parse { reason: &'static str, pos: usize },
    Unexpected { pos: usize },
    Overflow { pos: usize },
}

impl std::error::Error for Error {}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Eof => f.write_str("Unexpected end of file"),
            Error::Type { reason } => write!(f, "Type Mismatch: {}", reason),
            Error::Length { expected, actual } => write!(
                f,
                "Length Mismatch: Expected: {}, Actual: {}",
                expected, actual
            ),
            Error::Parse { reason, pos } => write!(f, "Parse Error at {}: {}", pos, reason),
            Error::Unexpected { pos } => write!(f, "Unexpected character at {}", pos),
            Error::Overflow { pos } => write!(f, "Numeric overflow occurred at {}", pos),
        }
    }
}

pub trait Decode<'buf>: Sized {
    fn decode<D>(decoder: D) -> Result<Self>
    where
        D: Decoder<'buf>;
}

pub fn parse<'buf, T>(buf: &'buf [u8]) -> Result<T>
where
    T: Decode<'buf>,
{
    let decoder = &mut BenDecoder { buf, pos: 0 };
    T::decode(decoder)
}

pub trait Decoder<'buf> {
    fn decode_dict<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'buf>;

    fn decode_list<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'buf>;

    fn decode_int(self) -> Result<i64>;

    fn decode_bytes(self) -> Result<&'buf [u8]>;
}

struct BenDecoder<'buf> {
    buf: &'buf [u8],
    pos: usize,
}

impl<'buf> Decoder<'buf> for &mut BenDecoder<'buf> {
    fn decode_dict<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'buf>,
    {
        if self.next_char()? != b'd' {
            return Err(Error::Parse {
                reason: "Expected Dict",
                pos: self.pos,
            });
        }

        let out = visitor.visit_dict(&mut *self)?;
        if self.next_char()? == b'e' {
            Ok(out)
        } else {
            Err(Error::Unexpected { pos: self.pos })
        }
    }

    fn decode_list<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'buf>,
    {
        if self.next_char()? != b'l' {
            return Err(Error::Parse {
                reason: "Expected List",
                pos: self.pos,
            });
        }

        let out = visitor.visit_list(&mut *self)?;

        if self.next_char()? == b'e' {
            Ok(out)
        } else {
            Err(Error::Unexpected { pos: self.pos })
        }
    }

    fn decode_int(self) -> Result<i64> {
        if self.next_char()? != b'i' {
            return Err(Error::Parse {
                reason: "Expected integer",
                pos: self.pos,
            });
        }

        self.parse_int_until(b'e')
    }

    fn decode_bytes(self) -> Result<&'buf [u8]> {
        if let b'0'..=b'9' = self.peek_char()? {
            // Ok
        } else {
            return Err(Error::Parse {
                reason: "Expected byte string",
                pos: self.pos,
            });
        }

        let len: usize = self
            .parse_int_until(b':')?
            .try_into()
            .map_err(|_| Error::Parse {
                reason: "Required positive length",
                pos: self.pos,
            })?;

        let bytes = self
            .buf
            .get(self.pos..)
            .and_then(|buf| buf.get(..len))
            .ok_or_else(|| Error::Eof)?;

        self.pos += len;
        Ok(bytes)
    }
}

impl<'buf> BenDecoder<'buf> {
    fn peek_char(&mut self) -> Result<u8> {
        self.buf.get(self.pos).copied().ok_or_else(|| Error::Eof)
    }

    fn next_char(&mut self) -> Result<u8> {
        let c = self.peek_char()?;
        self.pos += 1;
        Ok(c)
    }

    fn parse_int_until(&mut self, stop_char: u8) -> Result<i64> {
        let mut negative = false;

        if self.peek_char()? == b'-' {
            self.pos += 1;
            negative = true;
        }

        if let b'0'..=b'9' = self.peek_char()? {
            // Ok
        } else {
            return Err(Error::Unexpected { pos: self.pos });
        }

        let mut val: i64 = 0;
        loop {
            match self.next_char()? {
                c @ b'0'..=b'9' => {
                    val = val
                        .checked_mul(10)
                        .ok_or_else(|| Error::Overflow { pos: self.pos })?;
                    let c = (c - b'0') as i64;
                    val = val
                        .checked_add(c)
                        .ok_or_else(|| Error::Overflow { pos: self.pos })?;
                }
                c if c == stop_char => {
                    if negative {
                        val *= -1;
                    }
                    return Ok(val);
                }
                _ => return Err(Error::Unexpected { pos: self.pos }),
            }
        }
    }
}

pub trait Visitor<'buf>: Sized {
    type Value;

    fn visit_dict<A>(self, _v: A) -> Result<Self::Value>
    where
        A: Dict<'buf>,
    {
        Err(Error::Type {
            reason: "Dict not expected",
        })
    }

    fn visit_list<A>(self, _v: A) -> Result<Self::Value>
    where
        A: List<'buf>,
    {
        Err(Error::Type {
            reason: "List not expected",
        })
    }

    fn visit_bytes(self, _v: &'buf [u8]) -> Result<Self::Value> {
        Err(Error::Type {
            reason: "Byte string not expected",
        })
    }

    fn visit_int(self, _v: i64) -> Result<Self::Value> {
        Err(Error::Type {
            reason: "Integer not expected",
        })
    }
}

pub trait Dict<'buf> {
    fn next_entry<T>(&mut self) -> Result<Option<(&'buf [u8], T)>>
    where
        T: Decode<'buf>;
}

pub trait List<'buf> {
    fn next_element<T>(&mut self) -> Result<Option<T>>
    where
        T: Decode<'buf>;
}

impl<'buf> Dict<'buf> for &mut BenDecoder<'buf> {
    fn next_entry<T>(&mut self) -> Result<Option<(&'buf [u8], T)>>
    where
        T: Decode<'buf>,
    {
        if self.peek_char()? == b'e' {
            return Ok(None);
        }

        let key = self.decode_bytes()?;
        let value = T::decode(&mut **self)?;
        Ok(Some((key, value)))
    }
}

impl<'a, 'buf> List<'buf> for &mut BenDecoder<'buf> {
    fn next_element<T>(&mut self) -> Result<Option<T>>
    where
        T: Decode<'buf>,
    {
        if self.peek_char()? == b'e' {
            return Ok(None);
        }

        let v = T::decode(&mut **self)?;
        Ok(Some(v))
    }
}

impl<'buf> Decode<'buf> for &'buf [u8] {
    fn decode<D>(decoder: D) -> Result<Self>
    where
        D: Decoder<'buf>,
    {
        decoder.decode_bytes()
    }
}

impl<'buf> Decode<'buf> for i64 {
    fn decode<D>(decoder: D) -> Result<Self>
    where
        D: Decoder<'buf>,
    {
        decoder.decode_int()
    }
}

impl<'buf> Decode<'buf> for &'buf str {
    fn decode<D>(decoder: D) -> Result<Self>
    where
        D: Decoder<'buf>,
    {
        let bytes = decoder.decode_bytes()?;
        std::str::from_utf8(bytes).map_err(|_| Error::Type {
            reason: "Not a valid UTF-8 string",
        })
    }
}

////////////////// Impls //////////////////

macro_rules! tuple_impl {
    ($($t:ident),* ) => {
        impl<'buf, $( $t ),*> Decode<'buf> for ($( $t ),*)
        where
            $( $t: Decode<'buf> ),*
        {
            fn decode<D>(decoder: D) -> Result<Self>
            where
                D: Decoder<'buf>,
            {
                struct TheVisitor<$( $t ),*>(PhantomData<($( $t ),*)>);

                impl<'buf, $( $t ),*> Visitor<'buf> for TheVisitor<$( $t ),*>
                where
                    $( $t: Decode<'buf> ),*
                {
                    type Value = ($( $t ),*);

                    #[allow(unused)]
                    fn visit_list<A>(self, mut list: A) -> Result<Self::Value>
                    where
                        A: List<'buf>
                    {
                        Ok(($(
                            match list.next_element::<$t>()? {
                                Some(t) => t,
                                None => return Err(Error::Eof),
                            }
                        ),*))
                    }
                }

                decoder.decode_list(TheVisitor(PhantomData))
            }
        }
    }
}

tuple_impl!();
tuple_impl!(T0, T1);
tuple_impl!(T0, T1, T2);
tuple_impl!(T0, T1, T2, T3);
tuple_impl!(T0, T1, T2, T3, T4);
tuple_impl!(T0, T1, T2, T3, T4, T5);
tuple_impl!(T0, T1, T2, T3, T4, T5, T6);
tuple_impl!(T0, T1, T2, T3, T4, T5, T6, T7);
tuple_impl!(T0, T1, T2, T3, T4, T5, T6, T7, T8);
tuple_impl!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9);
tuple_impl!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
tuple_impl!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
tuple_impl!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
tuple_impl!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
tuple_impl!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
tuple_impl!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);

macro_rules! array_impl {
    ($len:literal => [$( $n:tt ),+]) => {
        impl<'buf, T> Decode<'buf> for [T; $len]
        where
            T: Decode<'buf>,
        {
            fn decode<D>(decoder: D) -> Result<Self>
            where
                D: Decoder<'buf>,
            {
                struct TheVisitor<T>(PhantomData<T>);

                impl<'buf, T> Visitor<'buf> for TheVisitor<T>
                where
                    T: Decode<'buf>,
                {
                    type Value = [T; $len];

                    fn visit_list<A>(self, mut list: A) -> Result<Self::Value>
                    where
                        A: List<'buf>
                    {
                        Ok([$(
                            match list.next_element()? {
                                Some(t) => t,
                                None => return Err(Error::Length { expected: $len, actual: $n }),
                            }
                        ),+])
                    }
                }

                decoder.decode_list(TheVisitor(PhantomData))
            }
        }
    }
}

array_impl!(1 => [0]);
array_impl!(2 => [0, 1]);
array_impl!(3 => [0, 1, 2]);
array_impl!(4 => [0, 1, 2, 3]);
array_impl!(5 => [0, 1, 2, 3, 4]);
array_impl!(6 => [0, 1, 2, 3, 4, 5]);
array_impl!(7 => [0, 1, 2, 3, 4, 5, 6]);
array_impl!(8 => [0, 1, 2, 3, 4, 5, 6, 7]);
array_impl!(9 => [0, 1, 2, 3, 4, 5, 6, 7, 8]);
array_impl!(10 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
array_impl!(11 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
array_impl!(12 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
array_impl!(13 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
array_impl!(14 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]);
array_impl!(15 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]);
array_impl!(16 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
array_impl!(17 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
array_impl!(18 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17]);
array_impl!(19 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18]);
array_impl!(20 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);
array_impl!(21 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
array_impl!(22 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21]);
array_impl!(23 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22]);
array_impl!(24 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23]);
array_impl!(25 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24]);
array_impl!(26 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25]);
array_impl!(27 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26]);
array_impl!(28 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27]);
array_impl!(29 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28]);
array_impl!(30 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29]);
array_impl!(31 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30]);
array_impl!(32 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31]);

macro_rules! list_impl {
    ($ty:ident, $fn:ident, Decode<'buf> $(+ $bounds:ident )* ) => {
        impl<'buf, T> Decode<'buf> for $ty<T>
        where
            T: Decode<'buf> $( + $bounds )*,
        {
            fn decode<D>(decoder: D) -> Result<Self>
            where
                D: Decoder<'buf>,
            {
                struct TheVisitor<T>(PhantomData<T>);

                impl<'buf, T> Visitor<'buf> for TheVisitor<T>
                where
                    T: Decode<'buf> $( + $bounds )*,
                {
                    type Value = $ty<T>;

                    fn visit_list<A>(self, mut list: A) -> Result<Self::Value>
                    where
                        A: List<'buf>
                    {
                        let mut out = $ty::new();
                        while let Some(t) = list.next_element()? {
                            out.$fn(t);
                        }
                        Ok(out)
                    }
                }

                decoder.decode_list(TheVisitor(PhantomData))
            }
        }
    }
}

list_impl!(Vec, push, Decode<'buf>);
list_impl!(VecDeque, push_back, Decode<'buf>);
list_impl!(HashSet, insert, Decode<'buf> + Hash + Eq);
list_impl!(BTreeSet, insert, Decode<'buf> + Ord);

macro_rules! map_impl {
    ($ty:ident) => {
        impl<'buf, T> Decode<'buf> for $ty<&'buf [u8], T>
        where
            T: Decode<'buf>,
        {
            fn decode<D>(decoder: D) -> Result<Self>
            where
                D: Decoder<'buf>,
            {
                struct TheVisitor<T>(PhantomData<T>);

                impl<'buf, T> Visitor<'buf> for TheVisitor<T>
                where
                    T: Decode<'buf>,
                {
                    type Value = $ty<&'buf [u8], T>;

                    fn visit_dict<A>(self, mut dict: A) -> Result<Self::Value>
                    where
                        A: Dict<'buf>,
                    {
                        let mut out = $ty::new();
                        while let Some((k, v)) = dict.next_entry()? {
                            out.insert(k, v);
                        }
                        Ok(out)
                    }
                }

                decoder.decode_list(TheVisitor(PhantomData))
            }
        }
    };
}

map_impl!(HashMap);
map_impl!(BTreeMap);
