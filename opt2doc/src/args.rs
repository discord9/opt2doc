use std::path::PathBuf;

use clap::{command, Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Repo dir to search for the cargo workspace.
    #[arg(short, long, default_value = ".")]
    pub repo: PathBuf,

    /// The path output files.
    #[arg(short, long, default_value = "target/opt2doc/")]
    pub output: PathBuf,

    /// Format to render.
    #[arg(short, long, value_enum)]
    pub render: RenderFormat,

    /// Name of the root option struct. Setting this will ignore all other options
    /// that are not accessible from the given root.
    #[arg(short, long)]
    pub root: Option<String>,

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
    /// Render a single-page HTML file with all options.
    Html,
    // TODO: support more formats
}
