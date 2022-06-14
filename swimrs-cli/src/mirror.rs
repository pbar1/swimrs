use std::path::PathBuf;

use anyhow::Result;
use async_channel::{unbounded, Receiver, Sender};
use chrono::NaiveDate;
use futures::future::join_all;
use metrics::gauge;
use metrics_exporter_prometheus::PrometheusBuilder;
use reqwest::{ClientBuilder, Proxy};
use swimrs::{
    common::Gender,
    usas::toptimes::{parse_top_times, TopTimesClient, TopTimesRequest},
};
use tracing::{debug, error};

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/102.0.5005.61/63 Safari/537.36";

pub async fn start_mirror(
    from_date: NaiveDate,
    to_date: NaiveDate,
    num_clients: u16,
) -> Result<()> {
    PrometheusBuilder::new().install()?;

    let (req_tx, req_rx) = unbounded();

    let mut handles = Vec::new();

    for i in 0..num_clients {
        // FIXME: Pass proxy settings as arguments
        let proxy = Proxy::all(format!("socks5://127.0.0.1:{}", 53000 + i))?;
        let builder = ClientBuilder::new().proxy(proxy).user_agent(USER_AGENT);
        let client = TopTimesClient::new(builder)?;

        let req_tx = req_tx.clone();
        let req_rx = req_rx.clone();
        let h = tokio::spawn(process_requests(client, req_tx, req_rx));
        handles.push(h);
    }

    let producer = tokio::spawn(produce_requests(from_date, to_date, req_tx));
    handles.push(producer);

    join_all(handles).await;

    Ok(())
}

// TODO: This is a logical place for a global rate limit! Let 'em drip
async fn produce_requests(
    from_date: NaiveDate,
    to_date: NaiveDate,
    req_tx: Sender<TopTimesRequest>,
) -> Result<()> {
    let age_range = [
        (Some(0), Some(7)),
        (Some(8), Some(8)),
        (Some(9), Some(9)),
        (Some(10), Some(10)),
        (Some(11), Some(11)),
        (Some(12), Some(12)),
        (Some(13), Some(13)),
        (Some(14), Some(14)),
        (Some(15), Some(15)),
        (Some(16), Some(16)),
        (Some(17), Some(17)),
        (Some(18), Some(18)),
        (Some(19), Some(19)),
        (Some(20), Some(20)),
        (Some(21), Some(21)),
        (Some(22), Some(22)),
        (Some(23), None),
    ];
    let num_days = (to_date - from_date).num_days() as usize + 1;

    for d in from_date.iter_days().take(num_days) {
        for (start_age, end_age) in age_range {
            let r_male = TopTimesRequest {
                gender: Gender::Male,
                from_date: d,
                to_date: d,
                start_age,
                end_age,
                ..TopTimesRequest::default()
            };
            let mut r_female = r_male.clone();
            r_female.gender = Gender::Female;

            if let Err(e) = req_tx.send(r_male).await {
                error!("error sending request into queue: {}", e);
            }
            if let Err(e) = req_tx.send(r_female).await {
                error!("error sending request into queue: {}", e);
            }
        }
    }

    Ok(())
}

// FIXME: Jitter
async fn process_requests(
    client: TopTimesClient,
    req_tx: Sender<TopTimesRequest>,
    req_rx: Receiver<TopTimesRequest>,
) -> Result<()> {
    client.populate_cookies().await?;
    debug!("populated cookies for client: {:?}", client);

    loop {
        gauge!("swimrs_mirror_request_queue_depth", req_tx.len() as f64);

        let req = match req_rx.recv().await {
            Ok(x) => x,
            Err(e) => {
                error!("error receiving from request queue: {}", e);
                continue;
            }
        };

        debug!("making request: {}", req);
        let req2 = req.clone();
        let html = match client.fetch_html(req).await {
            Ok(x) => x,
            Err(e) => {
                error!("error making request: {}", e);
                if let Err(e) = req_tx.send(req2).await {
                    error!("error sending request back into queue, DROPPING: {}", e);
                    continue;
                }
                continue;
            }
        };

        let gender = req2.gender.clone();
        let res = tokio::task::spawn_blocking(move || parse_top_times(html, gender));
        let times = match res.await {
            Ok(x) => x.unwrap(), // FIXME
            Err(e) => {
                error!("error parsing top times, DROPPING: {}", e);
                continue;
            }
        };

        debug!("{}: found {} times", req2, times.len());
        if times.len() < 1 {
            continue;
        }

        let mut path = PathBuf::new();
        path.push("results");
        path.push(req2.to_string().to_lowercase());
        if let Err(e) = tokio::fs::create_dir_all(&path).await {
            error!("error making directory {:?}: {}", path, e);
            continue;
        }
        path.push("results.csv");
        let mut writer = match csv::Writer::from_path(&path) {
            Ok(x) => x,
            Err(e) => {
                error!("error making csv writer for path {:?}: {}", path, e);
                continue;
            }
        };
        // TODO: Consider moving this into a blocking thread pool
        for t in times {
            if let Err(e) = writer.serialize(t) {
                error!("error writing csv line: {}", e);
                continue;
            }
        }
        if let Err(e) = writer.flush() {
            error!("error flushing csv writer: {}", e);
            continue;
        }
    }
}
