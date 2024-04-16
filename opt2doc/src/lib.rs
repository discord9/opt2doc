use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_jsonlines::json_lines;
use std::{
    collections::BTreeMap,
    fs::{create_dir_all, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use crate::socket::{get_socket_url, DocServerState};
mod socket;

pub use socket::DocClientState;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FieldMetadata {
    pub name: Option<String>,
    pub doc: Option<String>,
    pub ty: Vec<String>,
    pub default: Option<String>,
    pub deprecated: Option<String>,
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
#[derive(Debug, Serialize, Deserialize)]
pub struct DocOpts {
    /// default to store at `target/opt2doc/out.md`
    pub out_markdown: Option<PathBuf>,
}

impl Default for DocOpts {
    fn default() -> Self {
        let mut out_markdown = PathBuf::from("target/opt2doc/out.md");
        create_dir_all(out_markdown.parent().unwrap()).unwrap();
        DocOpts {
            out_markdown: Some(out_markdown),
        }
    }
}

impl DocOpts {
    /// read options either from default location of `opt2doc.toml` or from env var `OPT2DOC_CFG_FILE` determined config file
    pub fn read_opts() -> DocOpts {
        let cfg_loc = if let Ok(cfg_loc) = std::env::var("OPT2DOC_CFG_FILE") {
            PathBuf::from(cfg_loc)
        } else {
            PathBuf::from("opt2doc.toml")
        };

        let mut opt: Self = if let Ok(mut file) = OpenOptions::new().read(true).open(cfg_loc) {
            let mut buf = String::new();
            file.read_to_string(&mut buf)
                .expect("Can't read opt2doc.toml");
            toml::from_str(&buf).expect("Can't parse opt2doc.toml")
        } else {
            Default::default()
        };
        opt
    }
}

pub fn run_cargo_doc() {
    // first call cargo doc
    let output = std::process::Command::new("cargo")
        .arg("doc")
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
    if !output.success() {
        panic!("Failed to run `cargo doc`, exiting now");
    }
}

pub fn run_main() {
    let opt = DocOpts::read_opts();
    let mut server = DocServerState::new(&get_socket_url());
    let mut handle = std::process::Command::new("cargo")
        .arg("doc")
        .spawn()
        .expect("`cargo doc` command failed to start");
    let mut ret = Vec::new();

    //main loop
    loop {
        // if and only if `cargo doc` exit
        if handle.try_wait().unwrap().is_some() {
            break;
        }

        // first try to accept all incoming connections available
        while server.try_accept().is_some() {}

        ret.extend(server.try_recv());
    }

    let rendered = render_markdown(ret);

    // place it on same directory with tmp file

    let mut file = File::create(opt.out_markdown.unwrap()).unwrap();
    file.write_all(rendered.as_bytes()).unwrap();
}

fn render_markdown(items: Vec<CompsiteMetadata>) -> String {
    let items = items
        .into_iter()
        .map(|item| (item.name.clone(), item))
        .collect::<BTreeMap<_, _>>();

    // find out all root items, which is items that are not
    // referenced by any other items
    let root_items = items
        .iter()
        .filter(|(name, _)| {
            let typ = name;
            !items.iter().any(|(_, item)| {
                item.fields
                    .iter()
                    .any(|(_, field)| field.ty.last().unwrap() == *typ)
            })
        })
        .collect::<BTreeMap<_, _>>();

    // starting from root items, recursively find all items
    let mut out_markdown: Vec<CompsiteMetadata> = Vec::new();
    for (_, root) in root_items {
        // recursively find compsite items and expand them into new_fields
        let mut expaned_root = root.clone();
        let mut new_fields: Vec<(String, FieldMetadata)> = Vec::new();
        for (field_name, field) in &root.fields {
            expand_recur(field_name, field, &mut new_fields, &items, ".");
        }

        expaned_root.fields = new_fields;
        out_markdown.push(expaned_root);
    }
    out_markdown.iter().map(compsite_to_markdown).join("\n\n")
}

/// expand field with name delimitered by `delimiter`
pub fn expand_recur(
    field_name: &String,
    field: &FieldMetadata,
    new_fields: &mut Vec<(String, FieldMetadata)>,
    items: &BTreeMap<String, CompsiteMetadata>,
    delimiter: &str,
) {
    // items's name is their type, so if it's in items, it's a compsite data
    // so recursively find it's fields and append to new_fields
    if let Some(compsite) = items.get(field.ty.last().unwrap()) {
        // go through compsite's fields and expand them
        for (inner_field_name, inner_field) in &compsite.fields {
            let full_field_name = format!("{}{}{}", compsite.name, delimiter, inner_field_name);
            expand_recur(&full_field_name, inner_field, new_fields, items, delimiter);
        }
    } else {
        new_fields.push((field_name.to_string(), field.clone()));
    }
}

pub fn compsite_to_markdown(compsite: &CompsiteMetadata) -> String {
    let mut output = String::new();
    output.push_str(&format!("# {}\n", compsite.name));
    output.push_str(&format!("{}\n", compsite.doc));

    let table_header = [
        "| Key | Type | Default | Descriptions | Deprecated |\n",
        "| --- | ---- | ------- | ------------ | ---------- |\n",
    ];
    output.push_str(&table_header.join(""));
    for (field_name, field) in compsite.clone().fields {
        output.push_str(&format!(
            "|{}|{}|{}|{}|{}|\n",
            field_name,
            field.ty.join("."),
            field.default.unwrap_or("--".to_string()),
            field.doc.unwrap_or("--".to_string()),
            field.deprecated.unwrap_or("--".to_string())
        ));
    }
    output
}
