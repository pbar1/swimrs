use std::collections::HashMap;

use clap::clap_app;

mod usas;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = clap_app!(app =>
        (name: "swimrs")
        (version: "1.0")
        (author: "Pierce Bartine (pbar)")
        (about: "Swimming times data retrieval utility")
        (@setting SubcommandRequiredElseHelp)
        (@subcommand test =>
            (about: "Testing function")
        )
    )
    .get_matches();

    match matches.subcommand_name() {
        Some("test") => usas::test_fn().await,
        _ => panic!("impossible!"),
    }
}

async fn hello() -> Result<(), Box<dyn std::error::Error>> {
    let resp = reqwest::get("https://httpbin.org/ip")
        .await?
        .json::<HashMap<String, String>>()
        .await?;
    println!("{:#?}", resp);
    Ok(())
}
