use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_jsonlines::json_lines;
use std::{
    collections::BTreeMap,
    fs::{create_dir_all, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

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

pub fn read_from_tmp_file(opt: &DocOpts) -> Vec<CompsiteMetadata> {
    json_lines::<CompsiteMetadata, _>(opt.tmp_file.clone().unwrap())
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

pub fn run_main() {
    run_cargo_doc();
    // 1. read opt2doc.toml and parse into DocOpts
    // 2. read tmp file, and using json lines to
    // compact them into BTreeMap<(Name, Compsite)>
    // 3. output as markdown
    let opt = DocOpts::read_opts();
    let items = read_from_tmp_file(&opt);
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
    let out_markdown = out_markdown.iter().map(compsite_to_markdown).join("\n\n");
    // place it on same directory with tmp file
    let mut loc = opt.tmp_file.unwrap().parent().unwrap().to_path_buf();
    loc.push("out.md");

    let mut file = File::create(loc).unwrap();
    file.write_all(out_markdown.as_bytes()).unwrap();
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
