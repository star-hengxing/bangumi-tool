use clap::Parser;

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum Format {
    Json,
    Csv,
    All,
}

#[derive(Debug, Parser)]
#[command(name = "bangumi-tool", about = "Export Bangumi collection data")]
pub struct Args {
    /// Export format
    #[arg(short, long, value_enum, default_value = "all")]
    pub format: Format,

    /// Output directory
    #[arg(short, long, default_value = ".")]
    pub output: String,

    /// Enable debug logging (prints HTTP requests and responses)
    #[arg(long, default_value_t = false)]
    pub debug: bool,

    /// Disable cache and fetch everything fresh
    #[arg(long, default_value_t = false)]
    pub no_cache: bool,

    /// Fetch detailed info (episodes, progress) for each subject
    #[arg(long, default_value_t = false)]
    pub detail: bool,
}
