use bx::*;

fn main() {
    let f: Foo = bx::parse(b"d1:ale1:bi12ee").unwrap();
    println!("{:?}", f);
}

#[derive(Debug)]
struct Foo {
    a: (),
    b: i64,
}

impl<'a> Decode<'a> for Foo {
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self> {
        struct FooVisitor;

        impl<'buf> Visitor<'buf> for FooVisitor {
            type Value = Foo;

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
