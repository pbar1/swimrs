use std::{
    error::Error,
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};

use tokio::time::{sleep, Duration};

// https://kushaldas.in/posts/using-rust-to-access-internet-over-tor-via-socks-proxy.html
#[tokio::main]
async fn main() {
    let port = 9050;
    let cmd = Command::new("tor")
        .args(vec![
            "--SocksPort",
            format!("{}", port).as_str(),
            "--ControlPort",
            format!("{}", port + 1).as_str(),
            "--DisableDebuggerAttachment",
            "0",
        ])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let stdout = BufReader::new(cmd.stdout.unwrap());
    for line in stdout.lines() {
        if line.unwrap().contains("Bootstrapped 100% (done): Done") {
            break;
        }
    }

    let proxy_addr = format!("socks5://127.0.0.1:{}", port);
    let proxy = reqwest::Proxy::all(proxy_addr).unwrap();
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
