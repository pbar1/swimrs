use std::path::PathBuf;

use anyhow::Result;
use async_channel::{unbounded, Receiver, Sender};
use chrono::NaiveDate;
use reqwest::{ClientBuilder, Proxy};
use swimrs::{
    common::Gender,
    usas::toptimes::{parse_top_times, TopTimesClient, TopTimesRequest},
};
use tracing::{debug, error};

pub async fn start_mirror(
    from_date: NaiveDate,
    to_date: NaiveDate,
    num_clients: u16,
) -> Result<()> {
    let (req_tx, req_rx) = unbounded();
    let (wrt_tx, wrt_rx) = unbounded();

    let mut handles = Vec::new();

    for i in 0..num_clients {
        // FIXME: Pass proxy settings as arguments
        let proxy = Proxy::all(format!("socks://127.0.0.1:{}", 53000 + i))?;
        let builder = ClientBuilder::new().proxy(proxy);
        let client = TopTimesClient::new(builder)?;

        let req_tx = req_tx.clone();
        let req_rx = req_rx.clone();
        let wrt_tx = wrt_tx.clone();
        let h = tokio::spawn(process_requests(client, req_tx, req_rx, wrt_tx));
        handles.push(h);
    }

    // spawn parse/save processors
    let writer = tokio::task::spawn_blocking(process_writes(wrt_rx));

    let producer = tokio::spawn(produce_requests(from_date, to_date, req_tx));
    handles.push(producer);

    // join request producer

    // join request processors

    // join parse/save processors

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

            req_tx.send(r_male).await;
            req_tx.send(r_female).await;
        }
    }

    Ok(())
}

async fn process_requests(
    client: TopTimesClient,
    req_tx: Sender<TopTimesRequest>,
    req_rx: Receiver<TopTimesRequest>,
    wrt_tx: Sender<(TopTimesRequest, String)>,
) -> Result<()> {
    client.populate_cookies().await?;
    debug!("populated cookies for client: {:?}", client);

    loop {
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

        if let Err(e) = wrt_tx.send((req2, html)).await {
            error!("error sending html into write queue: {}", e);
        }
    }
}

// TODO: Might want to requeue failed writes
async fn process_writes(wrt_rx: Receiver<(TopTimesRequest, String)>) -> Result<()> {
    loop {
        let (req, html) = match wrt_rx.recv().await {
            Ok(x) => x,
            Err(e) => {
                error!("error receiving from write queue: {}", e);
                continue;
            }
        };

        let times = match parse_top_times(html, req.gender.clone()) {
            Ok(x) => x,
            Err(e) => {
                error!("error parsing top times, DROPPING: {}", e);
                continue;
            }
        };

        debug!("{}: found {} times", req, times.len());
        if times.len() < 1 {
            continue;
        }

        let mut path = PathBuf::new();
        path.push("results");
        path.push(req.to_string());
        if let Err(e) = tokio::fs::create_dir_all(&path).await {
            error!("error making directory {:?}: {}", path, e);
            continue;
        }
        path.push("results.csv");
        if let Err(e) = tokio::fs::write(&path, "todo").await {
            error!("error writing file {:?}: {}", path, e);
            continue;
        }
    }
}
