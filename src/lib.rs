pub mod de;
mod err;
mod parse;
mod impls;

pub use err::{Error, Result};

use de::Decode;
use parse::BenDecoder;

pub fn parse<'buf, T>(buf: &'buf [u8]) -> Result<T>
where
    T: Decode<'buf>,
{
    let decoder = &mut BenDecoder::new(buf);
    T::decode(decoder)
}
