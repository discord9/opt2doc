use derive_rust2md::Rust2Md;

fn main() {
    println!("Hello, world!");
}


/// Test opt
#[derive(Debug, Rust2Md)]
pub struct Opt{
    /// 1234241  
    name: String,
    #[rust2md(rename = "cfg_name", default="UTC", typ="String", doc="The timezone of the system")]
    id: usize
}