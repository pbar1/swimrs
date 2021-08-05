use std::error::Error;

use clap::{AppSettings, Clap};

mod usas;

#[derive(Clap)]
#[clap(version = "0.0.1", author = "Pierce Bartine")]
#[clap(about = "Swimming times retrieval utility")]
#[clap(setting = AppSettings::SubcommandRequiredElseHelp)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    #[clap(about = "USA Swimming Individual Times Search")]
    IndTimes,
    #[clap(about = "USA Swimming Top Times / Event Rank Search")]
    TopTimes,
    #[clap(about = "Mirror the USA Swimming SWIMS database")]
    Mirror(Mirror),
}

#[derive(Clap)]
struct Mirror {
    #[clap(short, long, default_value = "1")]
    #[clap(about = "Number of clients to use for processing requests [1, 10)")]
    concurrency: usize,
    #[clap(short, long)]
    #[clap(about = "Whether to execute the search requests")]
    dry_run: bool,
    #[clap(short, long)]
    #[clap(about = "Enables per-client rate limiting to 1 request/s")]
    enable_rate_limit: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let opts: Opts = Opts::parse();
    match opts.subcmd {
        SubCommand::IndTimes => usas::example_individual_times().await,
        SubCommand::TopTimes => usas::example_top_times().await,
        SubCommand::Mirror(m) => usas::mirror::mirror(m.concurrency, m.dry_run).await,
    }
}
