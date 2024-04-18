mod args;

use args::{Args, RenderFormat};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{create_dir_all, File},
    io::Write,
    path::PathBuf,
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

pub fn run_cargo_doc(repo: &PathBuf) {
    // first call cargo doc
    let output = std::process::Command::new("cargo")
        .arg("doc")
        .current_dir(repo)
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
    if !output.success() {
        panic!("Failed to run `cargo doc`, exiting now");
    }
}

pub fn run_main() {
    let args = Args::parse();

    let mut server = DocServerState::new(&get_socket_url());
    // first run `cargo doc --clean` to make sure we have the latest doc
    std::process::Command::new("cargo")
        .arg("clean")
        .arg("--doc")
        .current_dir(&args.repo)
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
    let mut handle = std::process::Command::new("cargo")
        .arg("doc")
        .current_dir(&args.repo)
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

    // render
    let render_output = match args.render {
        RenderFormat::None => {
            // no action needs
            // early return if no need to render
            return;
        }
        RenderFormat::Markdown => render_markdown(ret, &args.root),
        RenderFormat::Toml | RenderFormat::Yaml | RenderFormat::Html => {
            todo!("Not yet implemented")
        }
    };

    // place all markdown files on same directory with tmp file
    for (filename, content) in render_output {
        let full_path = args.output.clone().join(format!("{}.md", filename));
        create_dir_all(full_path.parent().unwrap()).unwrap();
        let mut file = File::create(full_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

/// Using toml_edit to generate example.toml with default values and comments
fn render_toml(items: Vec<CompsiteMetadata>, required_root: &Option<String>) -> String {
    todo!()
}

/// returns a key-value pair of filename and markdown content
fn render_markdown(
    items: Vec<CompsiteMetadata>,
    required_roots: &Option<Vec<String>>,
) -> Vec<(String, String)> {
    let items = items
        .into_iter()
        .map(|item| (item.name.clone(), item))
        .collect::<BTreeMap<_, _>>();

    // find out all root items, which is items that are not
    // referenced by any other items
    let mut root_items = items
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

    // filter root item if specified
    if let Some(required_roots) = required_roots {
        root_items.retain(|name, _| required_roots.contains(name));
    }

    // starting from root items, recursively find all items
    let mut metadata: Vec<CompsiteMetadata> = Vec::new();
    for (_, root) in root_items {
        // recursively find compsite items and expand them into new_fields
        let mut expaned_root = root.clone();
        let mut new_fields: Vec<(String, FieldMetadata)> = Vec::new();
        for (field_name, field) in &root.fields {
            expand_recur(field_name, field, &mut new_fields, &items, ".");
        }

        expaned_root.fields = new_fields;
        metadata.push(expaned_root);
    }
    metadata
        .iter()
        .map(|item| (item.name.clone(), compsite_to_markdown(item)))
        .collect()
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
            escape_markdown_in_cell_newline(&field.default.unwrap_or("--".to_string())),
            escape_markdown_in_cell_newline(&field.doc.unwrap_or("--".to_string())),
            escape_markdown_in_cell_newline(&field.deprecated.unwrap_or("--".to_string()))
        ));
    }
    output
}

/// Escape markdown in cell and replace newline with `  \\n`
/// TODO: make this more robust
fn escape_markdown_in_cell_newline(s: &str) -> String {
    s.replace('\n', "  \\n  ")
}
