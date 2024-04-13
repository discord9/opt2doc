use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::OpenOptions,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FieldMetadata {
    pub name: Option<String>,
    pub doc: Option<String>,
    pub ty: Vec<String>,
    pub default: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CompsiteMetadata {
    pub name: String,
    pub doc: String,
    pub fields: Vec<(String, FieldMetadata)>,
}

/// The whole documented struct, will be used to generate markdown documentation
#[derive(Default)]
pub struct Documented {
    pub types: BTreeMap<String, FieldMetadata>,
    /// will search from root_type and generate documentation for all the types that are reachable
    pub root_type: Option<String>,
}

/// The options for the `rust2md` derive macro
///
/// TODO: add useful options
#[derive(Default, Serialize, Deserialize)]
pub struct DocOpts {
    pub tmp_file: Option<String>,
}
impl DocOpts {
    pub fn read_opts() -> DocOpts {
        // run `pwd` to know the current working directory
        let res = std::process::Command::new("pwd")
            .output()
            .expect("Can't run pwd");
        // panic!("{:?}", res);
        let mut file = OpenOptions::new()
            .read(true)
            .open("rust2md.toml")
            .expect("Can't open rust2md.toml");
        let mut buf = String::new();
        file.read_to_string(&mut buf)
            .expect("Can't read rust2md.toml");
        let mut opt: Self = toml::from_str(&buf).expect("Can't parse rust2md.toml");
        if opt.tmp_file.is_none() {
            opt.tmp_file = Some(
                PathBuf::from_iter(["target", "rust2md", "tmp.json"])
                    .to_string_lossy()
                    .to_string(),
            );
        }
        opt
    }

    pub fn touch(&self){
        // touch and create(or recreate) the file
        let _ = OpenOptions::new()
            .create(true)
            .write(true)
            .open(self.tmp_file.clone().unwrap());
    }

    pub fn open_append_tmp_file(&self) -> std::io::Result<std::fs::File> {
        OpenOptions::new()
            .append(true)
            .open(self.tmp_file.clone().unwrap())
    }

    pub fn append_to_tmp_file(&self, content: &str) -> std::io::Result<()> {
        let mut file = self.open_append_tmp_file()?;
        file.write_all(content.as_bytes())
    }
}
