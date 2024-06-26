use std::path::PathBuf;

use clap::{command, Parser, ValueEnum};
use opt2doc_derive::Opt2Doc;

#[derive(Parser, Debug, Opt2Doc)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Optional name to operate on
    name: Option<String>,
    /// Repo dir to search for the cargo workspace.
    #[arg(long, default_value = ".")]
    #[opt2doc(default = ".")]
    pub repo: PathBuf,

    /// The path output files.
    #[arg(short, long, default_value = "target/opt2doc/")]
    #[opt2doc(default = "target/opt2doc/", typ = "String")]
    pub output: PathBuf,

    /// Format to render. Options are `Markdown/Toml/Yml/Html`
    #[arg(short, long, value_enum)]
    #[opt2doc(default = "None")]
    pub render: RenderFormat,

    /// Name of the root option struct. Setting this will ignore all other options
    /// that are not accessible from the given root.
    #[arg(long)]
    pub root: Option<Vec<String>>,

    /// The path of config file. E.g., `./opt2doc.toml`.
    #[arg(short, long)]
    pub config: Option<PathBuf>,
}

#[derive(Default, Parser, Debug, Clone, ValueEnum)]
pub enum RenderFormat {
    /// Do nothing. Only the JSON metadata file will be generated.
    #[default]
    None,
    /// Render a markdown file which contains a table of all options.
    Markdown,
    /// Render a toml file with all options set to default.
    Toml,
    /// Render a yaml file with all option set to default.
    Yaml,
    /// Render a single-page HTML file with all options.
    Html,
    // TODO: support more formats
}
