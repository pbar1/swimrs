use std::error::Error;

use tokio::time::{sleep, Duration};

// https://kushaldas.in/posts/using-rust-to-access-internet-over-tor-via-socks-proxy.html
#[tokio::main]
async fn main() {
    let proxy = reqwest::Proxy::all("socks5://127.0.0.1:9050").unwrap();
    let proxied_client = reqwest::Client::builder().proxy(proxy).build().unwrap();
    let regular_client = reqwest::Client::new();

    loop {
        println!("Proxied -> {}", check_ip(&proxied_client).await.unwrap());
        println!("Regular -> {}", check_ip(&regular_client).await.unwrap());
        sleep(Duration::from_secs(5)).await;
    }
}

async fn check_ip(client: &reqwest::Client) -> Result<String, Box<dyn Error>> {
    let res = client.get("http://checkip.amazonaws.com").send().await?;
    let status = res.status();
    let text = res.text().await?;
    Ok(format!("Status: {}, IP: {}", status, text.trim()))
}
