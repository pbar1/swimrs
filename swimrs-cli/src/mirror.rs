use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use async_channel::{unbounded, Receiver, Sender};
use chrono::NaiveDate;
use futures::future::join_all;
use log::{debug, error, info};
use metrics::{decrement_gauge, gauge, histogram, increment_gauge};
use metrics_exporter_prometheus::PrometheusBuilder;
use reqwest::{ClientBuilder, Proxy};
use swimrs::{
    common::Gender,
    usas::toptimes::{parse_top_times, TopTimesClient, TopTimesRequest},
};
use tokio::{
    fs, task,
    time::{sleep, Duration, Instant},
};

use crate::db::SqliteRequestDb;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/102.0.5005.61/63 Safari/537.36";

pub async fn start_mirror(
    from_date: NaiveDate,
    to_date: NaiveDate,
    num_clients: u16,
    db_url: &str,
) -> Result<()> {
    PrometheusBuilder::new().install()?;

    let db = Arc::new(SqliteRequestDb::new(db_url).await?);
    db.ensure_schema().await?;

    let (req_tx, req_rx) = unbounded();

    let mut handles = Vec::new();

    for i in 0..num_clients {
        // FIXME: Pass proxy settings as arguments
        let proxy = Proxy::all(format!("socks5://127.0.0.1:{}", 53000 + i))?;
        let builder = ClientBuilder::new().proxy(proxy).user_agent(USER_AGENT);
        let client = TopTimesClient::new(builder)?;

        let req_tx = req_tx.clone();
        let req_rx = req_rx.clone();
        let h = tokio::spawn(process_requests(client, req_tx, req_rx, db.clone()));
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

async fn process_requests(
    client: TopTimesClient,
    req_tx: Sender<TopTimesRequest>,
    req_rx: Receiver<TopTimesRequest>,
    db: Arc<SqliteRequestDb>,
) -> Result<()> {
    client.populate_cookies().await?;
    info!("populated cookies for client: {:?}", client);
    increment_gauge!("swimrs_mirror_ready_clients", 1.0);

    loop {
        gauge!("swimrs_mirror_request_queue_depth", req_tx.len() as f64);
        let start = Instant::now();

        let req = match req_rx.recv().await {
            Ok(x) => x,
            Err(e) => {
                error!("error receiving from request queue: {}", e);
                continue;
            }
        };
        let req_id = &req.to_string().to_lowercase();

        // FIXME
        if db.check_request_success(req_id).await.unwrap() {
            debug!("already made request: {}", req_id);
            continue;
        }

        debug!("making request: {}", req);
        let req2 = req.clone();
        match process_request(&client, req).await {
            Ok(l) => {
                debug!("found times for {}: {}", req_id, l);
                db.upsert_request_success(req_id, l, 0f64).await.unwrap(); // FIXME
            }
            Err(e) => {
                error!("error processing request {}: {}", req_id, e);
                db.upsert_request_error(req_id, &e.to_string(), 0f64)
                    .await
                    .unwrap(); // FIXME
                if let Err(e) = req_tx.send(req2).await {
                    error!("error sending request back into queue, DROPPING: {}", e);
                    continue;
                }
            }
        }

        let end = Instant::now();
        let delta = end.duration_since(start).as_secs();
        let delay = (rand::random::<f32>() * 5.0 + 5.0) as u64;
        if delta < delay {
            debug!("waiting for {} seconds", delay - delta);
            sleep(Duration::from_secs(delay - delta)).await;
        }
    }
}

async fn process_request(client: &TopTimesClient, req: TopTimesRequest) -> Result<u32> {
    let req2 = req.clone();
    let html = client.fetch_html(req).await?;

    let gender = req2.gender.clone();
    increment_gauge!("swimrs_mirror_request_active_count", 1.0);
    let start = Instant::now();
    let times = task::spawn_blocking(move || parse_top_times(html, gender)).await??;
    let end = Instant::now();
    decrement_gauge!("swimrs_mirror_request_active_count", 1.0);
    let req_duration = end.duration_since(start).as_secs_f64();
    histogram!("swimrs_mirror_request_duration", req_duration);

    debug!("{}: found {} times", req2, times.len());
    if times.is_empty() {
        return Ok(0);
    }
    let l = times.len() as u32;

    let mut path = PathBuf::new();
    path.push("results");
    path.push(req2.to_string().to_lowercase());
    fs::create_dir_all(&path).await?;
    path.push("results.csv");
    let mut writer = csv::Writer::from_path(&path)?;

    // TODO: Consider moving this into a blocking thread pool
    for t in times {
        writer.serialize(t)?;
    }
    writer.flush()?;

    Ok(l)
}
