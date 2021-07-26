use std::error::Error;

use clap::{App, AppSettings};

mod usas;

const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let matches = App::new("swimrs")
        .version(VERSION.unwrap_or("unknown"))
        .author("Pierce Bartine")
        .about("Swimming times data retrieval utility")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(App::new("toptimes").about("Top Times / Event Rank Search"))
        .subcommand(App::new("indtimes").about("Individual Times Search"))
        .subcommand(App::new("mirror").about("Download all times from USA Swimming"))
        .get_matches();

    match matches.subcommand_name() {
        Some("toptimes") => usas::example_top_times().await,
        Some("indtimes") => usas::example_individual_times().await,
        Some("mirror") => usas::mirror(),
        _ => panic!("impossible!"),
    }
}
