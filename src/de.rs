use crate::err::{Error, Result};

pub trait Decode<'buf>: Sized {
    fn decode<D>(decoder: D) -> Result<Self>
    where
        D: Decoder<'buf>;
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
