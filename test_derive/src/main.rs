use std::{fs::OpenOptions, sync::Mutex};

use derive_rust2md::{Rust2Md, doc_impl};
use once_cell::sync::Lazy;

fn main() {
    
    println!("Hello, world!");
}


/// Test opt
#[derive(Debug, Rust2Md)]
pub struct Opt {
    /// afa
    name: String,
    /// The timezone of the system
    #[rust2md(
        rename = "cfg_name",
        default = "UTC",
        typ = "String"
    )]
    id: usize,
}
