use darling::ast::NestedMeta;
use darling::FromMeta;
use opt2doc::{CompsiteMetadata, DocOpts, FieldMetadata};
use proc_macro::TokenStream;
use quote::quote;
use quote::ToTokens;
use std::collections::HashMap;
use std::sync::Mutex;
use syn::punctuated::Punctuated;
use syn::Lit::{self};
use syn::Meta::{self};
use syn::Result;
use syn::Token;
use syn::{parse_macro_input, Attribute, Error, Expr, ExprLit, Field, MetaNameValue};
/// options for the `opt2doc` derive macro
static OPT: once_cell::sync::Lazy<Mutex<DocOpts>> = once_cell::sync::Lazy::new(|| {
    Mutex::new({
        let opt = DocOpts::read_opts();
        opt.touch();
        opt
    })
});

/// `Rust2Md` is a derive macro that generates documentation for end user for i.e. cli options or
/// config file options.
///
/// use `opt2doc` on field more to generate markdown documentation.
///
/// i.e. the full attritube list are
///
/// `#[opt2doc(rename = "cfg_name", default="UTC", typ="String", doc="The timezone of the system")]`
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
#[proc_macro_derive(Opt2Doc, attributes(opt2doc))]
pub fn derive_doc(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);

    let mut fields = Vec::new();
    // let first deal with the struct fields
    if let syn::Data::Struct(s) = input.data {
        for field in s.fields {
            // 1. read `opt2doc` attribute's key val pairs
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

    let out_str = serde_json::to_string_pretty(&compsite).unwrap();
    // only generate doc if running `cargo doc`
    quote! {
        #[cfg(doc)]
        opt2doc_derive::doc_impl!(#out_str);
    }
    .into()
}

#[proc_macro]
pub fn doc_impl(input: TokenStream) -> TokenStream {
    // unescape the input since it's a string that got escaped
    let s: String = serde_json::from_str(&input.to_string()).unwrap();

    let compsite: CompsiteMetadata = serde_json::from_str(&s).unwrap();
    OPT.lock().unwrap().insert_type(compsite);
    quote! {}.into()
}

fn get_attrs_from_field(field: &Field) -> Result<FieldMetadata> {
    let mut doc = parse_opt2doc_attrs(field)?;
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

    if doc.deprecated.is_none() {
        doc.deprecated = Some(get_deprecated_comment(&field.attrs));
    }

    Ok(doc)
}

/// a full example of all the attributes:
///  
/// `#[opt2doc(rename = "cfg_name", default="UTC", type="String", doc="The timezone of the system")]`
fn parse_opt2doc_attrs(field: &Field) -> Result<FieldMetadata> {
    // first get attribute with name of `opt2doc`
    let mut doc = FieldMetadata::default();
    let attr = if let Some(attr) = field
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("opt2doc"))
    {
        attr
    } else {
        return Ok(FieldMetadata::default());
    };
    let attr_meta_list = if let Meta::List(list) = &attr.meta {
        list
    } else {
        return Err(Error::new_spanned(attr, "expected #[opt2doc(...)]"));
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
        let trimmed = line.trim_start();
        let trimmed = trimmed.trim_end();
        *line = trimmed;
    }

    lines.join("\n")
}

/// Extracts the [`deprecated`] attribute from the given attributes.
///
/// Returns:
/// - "true": if it's `#[deprecated]`
/// - "message": if it's `#[deprecated = "message"]`
/// - "since" and "note": if it's `#[deprecated(since = "version", note = "message")]`
///
/// [`deprecated`]: https://doc.rust-lang.org/reference/attributes/diagnostics.html#the-deprecated-attribute
fn get_deprecated_comment(attrs: &[Attribute]) -> String {
    let message_parts = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("deprecated"))
        .filter_map(|attr| match &attr.meta {
            Meta::NameValue(MetaNameValue { value, .. }) => {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) = value
                {
                    Some(s.value())
                } else {
                    None
                }
            }
            Meta::List(list) => {
                let mut since = String::new();
                let mut note = String::new();

                for nested in list
                    .parse_args_with(Punctuated::<MetaNameValue, Token![,]>::parse_terminated)
                    .ok()?
                {
                    let MetaNameValue { path, value, .. } = nested;
                    let value = if let Expr::Lit(ExprLit {
                        lit: Lit::Str(s), ..
                    }) = value
                    {
                        s.value()
                    } else {
                        return None;
                    };
                    if path.is_ident("since") {
                        since = format!("since: {}", value);
                    } else if path.is_ident("note") {
                        note = format!("note: {}", value);
                    }
                }

                Some(format!("{}, {}", since, note))
            }
            _ => Some("true".to_string()),
        })
        .collect::<Vec<_>>();

    message_parts.join("\n")
}
