use bx::de::{Decode, Decoder, Dict, Visitor};
use bx::{Error, Result};

fn main() {
    let f: Foo = bx::parse(b"d1:b3:abc1:ai12ee").unwrap();
    println!("{:?}", f);
}

#[derive(Debug)]
struct Foo<'a> {
    id: &'a [u8],
    b: i64,
}

impl<'a> Decode<'a> for Foo<'a> {
    fn decode<D>(decoder: D) -> Result<Self>
    where
        D: Decoder<'a>,
    {
        struct FooVisitor;

        impl<'buf> Visitor<'buf> for FooVisitor {
            type Value = Foo<'buf>;

            fn visit_dict<A>(self, mut dict: A) -> Result<Self::Value>
            where
                A: Dict<'buf>,
            {
                if let Some((b"a", id)) = dict.next_entry()? {
                    if let Some((b"b", b)) = dict.next_entry()? {
                        return Ok(Foo { id, b });
                    }
                }
                Err(Error::Eof)
            }
        }

        decoder.decode_dict(FooVisitor)
    }
}
