#![allow(unused)]
// TODO: use trybuild for macro test
use opt2doc_derive::Opt2Doc;

fn main() {
    println!("Hello, world!");
}

/// Test opt
#[derive(Debug, Opt2Doc)]
pub struct Opt {
    /// afa a
    name: String,
    /// The timezone of the system
    #[opt2doc(rename = "cfg_name", default = "UTC", typ = "String")]
    id: usize,
    inner: InnerOpt,
    deprecated: Deprecated,
}

#[derive(Debug, Opt2Doc)]
pub struct InnerOpt {
    cfg: bool,
    ttl: usize,
}

#[derive(Debug, Opt2Doc)]
pub struct Deprecated {
    #[deprecated]
    plain: String,
    #[deprecated = "some deprecate message"]
    with_message: String,
    #[deprecated(since = "v0.1.1", note = "another deprecate message")]
    since_and_note: String,
}
