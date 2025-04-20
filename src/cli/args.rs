use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CommandLineArgs {
    /// Whether alphadep should write archive to current directory and terminate program
    #[arg(long, default_value_t = false)]
    pub(crate) write_archive: bool,
}
