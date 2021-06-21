use std::collections::HashMap;
use std::error::Error;

use clap::clap_app;

mod usas;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let matches = clap_app!(app =>
        (name: "swimrs")
        (version: "1.0")
        (author: "Pierce Bartine (pbar)")
        (about: "Swimming times data retrieval utility")
        (@setting SubcommandRequiredElseHelp)
        (@subcommand toptimes =>
            (about: "Top Times / Event Rank Search")
        )
        (@subcommand indtimes =>
            (about: "Individual Times Search")
        )
    )
    .get_matches();

    match matches.subcommand_name() {
        Some("toptimes") => usas::example_top_times().await,
        Some("indtimes") => usas::example_individual_times().await,
        _ => panic!("impossible!"),
    }
}

async fn hello() -> Result<(), Box<dyn Error>> {
    let resp = reqwest::get("https://httpbin.org/ip")
        .await?
        .json::<HashMap<String, String>>()
        .await?;
    println!("{:#?}", resp);
    Ok(())
}
