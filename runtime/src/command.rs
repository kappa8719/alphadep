use clap::{Args, Parser, Subcommand};

#[derive(Args, Debug)]
pub struct ExtractArgs {
    #[arg(long = "archive")]
    pub archive: String,

    #[arg(long = "destination")]
    pub destination: Option<String>,

    #[arg(long = "overwrite", default_value_t = false)]
    pub overwrite: bool,
}

#[derive(Args, Debug)]
pub struct ExecuteArgs {
    #[arg(long = "silent", default_value_t = false)]
    pub(crate) silent: bool,
    #[arg(long = "directory", default_value_t = (".".to_string()))]
    pub directory: String,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Extract(ExtractArgs),
    Build,
    Execute(ExecuteArgs),
}

#[derive(Parser, Debug)]
#[command(version, about)]
#[command(propagate_version = true)]
pub struct CommandLineArgs {
    #[clap(subcommand)]
    pub command: Commands,
}
