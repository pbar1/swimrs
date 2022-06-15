mod db;
mod mirror;

use anyhow::Result;
use chrono::NaiveDate;
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
    /// Starting date in the range to mirror
    from_date: NaiveDate,
    /// Ending date in the range to mirror
    to_date: NaiveDate,
    /// Number of unique HTTP clients to send requests with
    #[clap(long, default_value = "1")]
    clients: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let cli = Cli::parse();
    match &cli.command {
        Commands::Mirror(args) => {
            mirror::start_mirror(args.from_date, args.to_date, args.clients).await?
        }
    }

    Ok(())
}
