use bx::*;

fn main() {
    let f: Foo = bx::parse(b"d1:al3:abc3:abci50ee1:bi12ee").unwrap();
    println!("{:?}", f);
}

#[derive(Debug)]
struct Foo<'a> {
    a: (&'a str, &'a [u8], i64),
    b: i64,
}

impl<'a> Decode<'a> for Foo<'a> {
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self> {
        struct FooVisitor;

        impl<'buf> Visitor<'buf> for FooVisitor {
            type Value = Foo<'buf>;

            fn visit_dict(self, mut dict: DictAccess<'_, 'buf>) -> Result<Self::Value> {
                if let Some((b"a", v)) = dict.next_entry()? {
                    let a = v;
                    if let Some((b"b", v)) = dict.next_entry()? {
                        let b = v;
                        return Ok(Foo { a, b });
                    }
                }
                Err(Error::Eof)
            }
        }

        decoder.decode_dict(FooVisitor)
    }
}
