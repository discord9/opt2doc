use std::collections::HashMap;
use std::sync::Mutex;
use darling::ast::NestedMeta;
use darling::FromMeta;
use proc_macro::TokenStream;
use quote::quote;
use quote::ToTokens;
use rust2md::{CompsiteMetadata, DocOpts, FieldMetadata};
use syn::Lit::{self, Str};
use syn::Meta::{self, NameValue};
use syn::{parse_macro_input, Attribute, Error, Expr, ExprLit, Field, MetaNameValue};
use syn::{MetaList, Result};
/// options for the `rust2md` derive macro
static OPT: once_cell::sync::Lazy<Mutex<DocOpts>> =
    once_cell::sync::Lazy::new(|| Mutex::new(DocOpts::read_opts()));

/// `Rust2Md` is a derive macro that generates documentation for end user for i.e. cli options or
/// config file options.
///
/// use `rust2md` on field more to generate markdown documentation.
///
/// i.e. the full attritube list are
///
/// `#[rust2md(rename = "cfg_name", default="UTC", typ="String", doc="The timezone of the system")]`
///
/// where `rename` means the name of the
/// option in the config file and `default` is the default value of the option.
///
/// `type = "String"` is the type of the option and `doc` is the documentation of the option.
///
/// `doc` is the docmuemntation of the option.
///
/// if any of those is missing, this macro will try it's best to extract the information from the
/// struct field definition.
#[proc_macro_derive(Rust2Md, attributes(rust2md))]
pub fn derive_doc(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);

    let mut fields = Vec::new();
    // let first deal with the struct fields
    if let syn::Data::Struct(s) = input.data {
        for field in s.fields {
            // 1. read `rust2md` attribute's key val pairs
            let raw_doc = match get_attrs_from_field(&field) {
                Ok(v) => v,
                Err(e) => return e.to_compile_error().into(),
            };
            fields.push((raw_doc.name.clone().unwrap(), raw_doc));
        }
    }

    let compsite = CompsiteMetadata {
        name: input.ident.to_string(),
        doc: get_doc_comment(&input.attrs),
        fields,
    };
    
    let opt = OPT.lock().unwrap();
    serde_jsonlines::write_json_lines(opt.tmp_file.clone().unwrap(), vec![compsite]).unwrap();
    quote! {}.into()
}

fn get_attrs_from_field(field: &Field) -> Result<FieldMetadata> {
    let mut doc = parse_rust2md_attrs(field)?;
    if doc.name.is_none() {
        doc.name = Some(field.ident.as_ref().unwrap().to_string());
    }

    if doc.doc.is_none() {
        doc.doc = Some(get_doc_comment(&field.attrs));
    }

    if doc.ty.is_empty() {
        doc.ty = if let syn::Type::Path(t) = &field.ty {
            t.path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect()
        } else {
            return Err(Error::new_spanned(
                field.ty.clone().into_token_stream(),
                "type is expected to be a path",
            ));
        };
    }
    Ok(doc)
}

/// a full example of all the attributes:
///  
/// `#[rust2md(rename = "cfg_name", default="UTC", type="String", doc="The timezone of the system")]`
fn parse_rust2md_attrs(field: &Field) -> Result<FieldMetadata> {
    // first get attribute with name of `rust2md`
    let mut doc = FieldMetadata::default();
    let attr = if let Some(attr) = field
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("rust2md"))
    {
        attr
    } else {
        return Ok(FieldMetadata::default());
    };
    let attr_meta_list = if let Meta::List(list) = &attr.meta {
        list
    } else {
        return Err(Error::new_spanned(attr, "expected #[rust2md(...)]"));
    };
    let meta_list = NestedMeta::parse_meta_list(attr_meta_list.tokens.clone())?;

    let kv_pairs: HashMap<String, String> = HashMap::from_list(&meta_list)?;

    for (k, v) in kv_pairs.iter() {
        match k.as_str() {
            "rename" => doc.name = Some(v.clone()),
            "default" => doc.default = Some(v.clone()),
            "typ" => doc.ty = vec![v.clone()],
            "doc" => doc.doc = Some(v.clone()),
            _ => {}
        }
    }
    Ok(doc)
}

/// Extracts the doc comment from the given attributes.
fn get_doc_comment(attrs: &[Attribute]) -> String {
    let comment_parts: Vec<_> = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .filter_map(|attr| {
            if let Meta::NameValue(MetaNameValue {
                value:
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(s), ..
                    }),
                ..
            }) = attr.meta.clone()
            {
                Some(s.value())
            } else {
                // non #[doc = "..."] attributes are not our concern
                // we leave them for rustc to handle
                None
            }
        })
        .collect();
    let mut lines: Vec<&str> = comment_parts
        .iter()
        .skip_while(|s| s.trim().is_empty())
        .flat_map(|s| s.split('\n'))
        .collect();
    for line in lines.iter_mut() {
        if line.starts_with(' ') {
            *line = &line[1..];
        }
    }

    comment_parts.join("\n")
}
