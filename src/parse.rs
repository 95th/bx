use crate::de::{Decode, Decoder, Dict, List, Visitor};
use crate::err::{Error, Result};

pub struct BenDecoder<'buf> {
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

        self.parse_i64(b'e')
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

        let len = self.parse_usize(b':')?;

        match self.buf.get(self.pos..self.pos + len) {
            Some(buf) => {
                self.pos += len;
                Ok(buf)
            }
            None => Err(Error::Eof),
        }
    }
}

impl<'buf> BenDecoder<'buf> {
    pub fn new(buf: &'buf [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn peek_char(&mut self) -> Result<u8> {
        self.buf.get(self.pos).copied().ok_or_else(|| Error::Eof)
    }

    fn next_char(&mut self) -> Result<u8> {
        let c = self.peek_char()?;
        self.pos += 1;
        Ok(c)
    }

    fn parse_usize(&mut self, stop_char: u8) -> Result<usize> {
        if let b'0'..=b'9' = self.peek_char()? {
            // Ok
        } else {
            return Err(Error::Unexpected { pos: self.pos });
        }

        let mut val: usize = 0;
        loop {
            match self.next_char()? {
                c @ b'0'..=b'9' => {
                    let digit = (c - b'0') as usize;
                    match val.checked_mul(10).and_then(|n| n.checked_add(digit)) {
                        Some(n) => val = n,
                        None => return Err(Error::Overflow { pos: self.pos }),
                    }
                }
                c if c == stop_char => return Ok(val),
                _ => return Err(Error::Unexpected { pos: self.pos }),
            }
        }
    }

    fn parse_i64(&mut self, stop_char: u8) -> Result<i64> {
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
                    let digit = (c - b'0') as i64;
                    match val.checked_mul(10).and_then(|n| n.checked_add(digit)) {
                        Some(n) => val = n,
                        None => return Err(Error::Overflow { pos: self.pos }),
                    }
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
