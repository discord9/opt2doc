use serde::{Deserialize, Serialize};
use std::{
    fs::{create_dir_all, OpenOptions},
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FieldMetadata {
    pub name: Option<String>,
    pub doc: Option<String>,
    pub ty: Vec<String>,
    pub default: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompsiteMetadata {
    pub name: String,
    pub doc: String,
    pub fields: Vec<(String, FieldMetadata)>,
}

/// The options for the `opt2doc` derive macro
///
/// TODO: add useful options
#[derive(Default, Serialize, Deserialize)]
pub struct DocOpts {
    /// default to store at `target/opt2doc/tmp.json`
    pub tmp_file: Option<Box<Path>>,
}
impl DocOpts {
    pub fn read_opts() -> DocOpts {
        let mut opt: Self = if let Ok(mut file) = OpenOptions::new().read(true).open("opt2doc.toml")
        {
            let mut buf = String::new();
            file.read_to_string(&mut buf)
                .expect("Can't read opt2doc.toml");
            toml::from_str(&buf).expect("Can't parse opt2doc.toml")
        } else {
            Default::default()
        };
        if opt.tmp_file.is_none() {
            opt.tmp_file = Some(PathBuf::from_iter(["target", "opt2doc", "tmp.json"]).into());
        }
        opt
    }

    /// Create the tmp file and it's parent directory(if needed)
    pub fn touch(&self) {
        // touch and create(or recreate) the file and it's directory
        create_dir_all(self.tmp_file.clone().unwrap().parent().unwrap())
            .expect("Create opt2doc folder in target directory");
        let _ = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.tmp_file.clone().unwrap())
            .unwrap();
    }

    // TODO: use interprocess communication to avoid file IO
    pub fn insert_type(&self, compsite: CompsiteMetadata) {
        serde_jsonlines::append_json_lines(self.tmp_file.clone().unwrap(), vec![compsite]).unwrap();
    }
}
