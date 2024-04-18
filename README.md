This reposity contains two packages: 
- `opt2doc` is a binary that can be run as cargo subcommand
- `opt2doc_derive` is a library that can be used to derive `opt2doc` for your own types

# Command Args

| Key | Type | Default | Descriptions | Deprecated |
| --- | ---- | ------- | ------------ | ---------- |
|name|Option|--|Optional name to operate on||
|repo|PathBuf|.|Repo dir to search for the cargo workspace.||
|output|PathBuf|target/opt2doc/|The path output files.||
|render|RenderFormat|--|Format to render.||
|root|Option|--|Name of the root option struct. Setting this will ignore all other options
that are not accessible from the given root.||
|config|Option|--|The path of config file. E.g., `./opt2doc.toml`.||