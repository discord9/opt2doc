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
}

#[derive(Debug, Opt2Doc)]
pub struct InnerOpt {
    cfg: bool,
    ttl: usize,
}
