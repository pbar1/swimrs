mod mirror;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Mirror the USA Swimming times database
    Mirror(MirrorArgs),
}

#[derive(Args)]
struct MirrorArgs {
    /// Number of unique HTTP clients to send requests with
    #[clap(long, default_value = "1")]
    clients: usize,
    /// If enabled, HTTP requests will not be sent
    #[clap(long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Mirror(_) => mirror::start_mirror().await?,
    }
    Ok(())
}
