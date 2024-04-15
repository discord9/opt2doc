use itertools::Itertools;
use opt2doc::{CompsiteMetadata, DocOpts, FieldMetadata};
use serde_jsonlines::json_lines;
use std::{collections::BTreeMap, fs::File, io::Write};

fn main() {
    // first call cargo doc
    let output = std::process::Command::new("cargo")
        .arg("doc")
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
    if !output.success() {
        println!("Failed to run `cargo doc`, exiting now");
        return;
    }
    // 1. read opt2doc.toml and parse into DocOpts
    // 2. read tmp file, and using json lines to
    // compact them into BTreeMap<(Name, Compsite)>
    // 3. output as markdown
    let opt = DocOpts::read_opts();
    let items = json_lines::<CompsiteMetadata, _>(opt.tmp_file.clone().unwrap())
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
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
fn expand_recur(
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
        let full_field_name = format!("{}{}{}", compsite.name, delimiter, field_name);
        for field in &compsite.fields {
            expand_recur(&full_field_name, &field.1, new_fields, items, delimiter);
        }
    } else {
        new_fields.push((field_name.to_string(), field.clone()));
    }
}

fn compsite_to_markdown(compsite: &CompsiteMetadata) -> String {
    let mut output = String::new();
    output.push_str(&format!("# {}\n", compsite.name));
    output.push_str(&format!("{}\n", compsite.doc));

    let table_header = [
        "| Key | Type | Default | Descriptions |\n",
        "| --- | -----| ------- | ----------- |\n",
    ];
    output.push_str(&table_header.join(""));
    for (field_name, field) in compsite.clone().fields {
        output.push_str(&format!(
            "|{}|{}|{}|{}|\n",
            field_name,
            field.ty.join("."),
            field.default.unwrap_or("--".to_string()),
            field.doc.unwrap_or("--".to_string()),
        ));
    }
    output
}