use std::{
    collections::VecDeque, error::Error, process::Stdio, thread, time::Duration as StdDuration,
};

use async_channel::Sender;
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use dashmap::DashSet;
use futures::{future::join_all, StreamExt};
use governor::{Quota, RateLimiter};
use log::{debug, error, info, trace, warn};
use metrics::{
    decrement_gauge, gauge, histogram, increment_counter, increment_gauge, register_counter,
    register_histogram,
};
use metrics_exporter_prometheus::PrometheusBuilder;
use nonzero_ext::nonzero;
use rand::{seq::SliceRandom, thread_rng, Rng};
use signal_hook::consts::signal::*;
use signal_hook_tokio::Signals;
use stopwatch::Stopwatch;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    select,
    task::JoinHandle,
};

use crate::usas::{
    model::{
        Course, Gender, Stroke, SwimEvent, Zone, COURSES, DISTANCES, IND_STROKES, VALID_EVENTS,
        ZONES,
    },
    toptimes,
    toptimes::{TopTimesClient, TopTimesRequest},
};

// TODO make sure to handle Gender in the model
// https://stackoverflow.com/questions/51044467/how-can-i-perform-parallel-asynchronous-http-get-requests-with-reqwest
pub async fn mirror(concurrency: usize, dry_run: bool) -> Result<(), Box<dyn Error>> {
    // Initialize Prometheus metrics endpoint at localhost:9000
    // tracing_subscriber::fmt::init();
    let builder = PrometheusBuilder::new();
    builder
        .install()
        .expect("failed to install Prometheus recorder");

    // Launch Tor SOCKS proxies for use by our clients
    let mut tor_handles: Vec<JoinHandle<()>> = Vec::new();
    for i in 0..concurrency {
        let handle = tokio::spawn(async move {
            let stdout = Command::new("tor")
                .args(vec![
                    "--torrc-file",
                    "does_not_exist",
                    "--allow-missing-torrc",
                    "--SocksPort",
                    format!("{}", 53000 + i).as_str(),
                    "--DataDirectory",
                    format!("tordata/{}", i).as_str(),
                ])
                .stdout(Stdio::piped())
                .spawn()
                .unwrap()
                .stdout
                .unwrap();
            let mut lines = BufReader::new(stdout).lines();
            while let Some(line) = lines.next_line().await.unwrap() {
                if line.contains("Bootstrapped 100% (done): Done") {
                    info!("Started Tor proxy {}", i);
                    increment_gauge!("tor_proxies", 1.0);
                    break;
                }
            }
        });
        tor_handles.push(handle)
    }
    join_all(tor_handles).await;

    // Create a pool of clients each proxied through Tor
    let (tx, rx) = async_channel::unbounded();
    let mut client_handles: Vec<JoinHandle<()>> = Vec::new();
    for i in 0..concurrency {
        // let lim = RateLimiter::direct(Quota::per_minute(nonzero!(90u32)));
        let tx2 = tx.clone();
        let rx2 = rx.clone();
        let proxy_addr = format!("socks5://127.0.0.1:{}", 53000 + i);
        let client = toptimes::TopTimesClient::new(proxy_addr.as_str()).unwrap();
        let handle = tokio::spawn(async move {
            client.populate_cookies().await.unwrap();
            loop {
                select! {
                    Ok(r) = rx2.recv() => {
                        debug!("Making request on client {}: {}", i, r);
                        let client2 = client.clone();
                        make_request(client2, r, dry_run, tx2.clone()).await;
                        let depth = tx2.len();
                        gauge!("mirror_request_queue_depth", depth as f64);
                        trace!("Request queue depth: {}", depth);
                        // lim.until_ready().await;
                    }
                    else => {
                        break;
                    }
                }
            }
        });
        client_handles.push(handle);
    }

    // Expand requests, shuffle them, and send them into the channel for processing
    // let root_req = toptimes::TopTimesRequest {
    //     gender: Gender::Male,
    //     distance: 0,
    //     stroke: Stroke::All,
    //     course: Course::All,
    //     from_date: NaiveDate::from_ymd(2020, 1, 1),
    //     to_date: NaiveDate::from_ymd(2020, 12, 31),
    //     start_age: Some(0),
    //     end_age: None,
    //     max_results: 5000,
    //     ..toptimes::TopTimesRequest::default()
    // };
    // let mut requests = atomize(root_req, true, true, true, true, true, false);
    let mut requests = gen_requests_smart(
        NaiveDate::from_ymd(2020, 1, 1),
        NaiveDate::from_ymd(2020, 12, 31),
    );
    debug!("Generated {} total requests", requests.len());
    let set = DashSet::new();
    for entry in glob::glob("**/result.csv").expect("failed to read glob pattern") {
        match entry {
            Ok(path) => {
                set.insert(
                    path.to_str()
                        .unwrap()
                        .to_string()
                        .replace("results/", "")
                        .replace("/result.csv", ""),
                );
            }
            Err(e) => error!("Glob result error: {:?}", e),
        };
    }
    let mut rng = rand::thread_rng();
    requests.shuffle(&mut rng);
    for r in requests {
        if !set.contains(r.to_string().as_str()) {
            tx.send(r).await?;
        }
    }
    // tx.close();

    // TODO: Start HTTP server to expose channel depth
    let signals = Signals::new(&[SIGUSR1])?;
    let signals_task = tokio::spawn(handle_signals(signals, tx.clone()));
    client_handles.push(signals_task);

    // Begin processing all of the requests
    join_all(client_handles).await;

    Ok(())
}

async fn make_request(
    client: TopTimesClient,
    req: TopTimesRequest,
    dry_run: bool,
    tx: Sender<TopTimesRequest>,
) {
    if dry_run {
        return;
    }
    let dir = format!("results/{}/", req);
    tokio::fs::create_dir_all(dir.clone()).await.unwrap();
    let req2 = req.clone();
    let mut sw = Stopwatch::start_new();
    let result = client.search(req).await;
    sw.stop();
    match result {
        Ok(times) => {
            let times_found = times.len();
            histogram!("mirror_times_found", times_found as f64);
            histogram!("mirror_request_duration_seconds", sw.elapsed(), "result" => "success");
            info!("Found {} times for request: {}", times.len(), req2);
            if times_found >= 4900 {
                increment_counter!("mirror_large_results");
                warn!("Request possibly too large, writing warning: {}", req2);
                tokio::fs::File::create(dir.clone() + "warning.txt")
                    .await
                    .unwrap();
            }
            let mut wtr = csv::Writer::from_path(dir + "result.csv").unwrap();
            for t in times {
                wtr.serialize(t).unwrap();
            }
            wtr.flush().unwrap();
        }
        Err(e) => {
            histogram!("mirror_request_duration_seconds", sw.elapsed(), "result" => "error");
            error!("Error for request {}: {}", req2, e);
            tx.send(req2).await.unwrap();
        }
    }
}

/// Divides the given TopTimesRequest into the smallest possible chunks that obey the given parameters.
fn atomize(
    req: TopTimesRequest,
    by_course: bool,
    by_stroke: bool,
    by_distance: bool,
    by_date: bool,
    by_age: bool,
    by_zone: bool,
) -> Vec<TopTimesRequest> {
    let mut output: Vec<TopTimesRequest> = Vec::new();
    let mut queue: VecDeque<TopTimesRequest> = VecDeque::new();
    queue.push_back(req);
    while let Some(r) = queue.pop_front() {
        let r2 = r.clone();
        let divided = divide(
            r,
            by_course,
            by_stroke,
            by_distance,
            by_date,
            by_age,
            by_zone,
        );
        if divided.is_empty() {
            output.push(r2);
            continue;
        }
        queue.extend(divided);
    }
    output
}

/// Divides the given TopTimesRequest into smaller chunks.
/// Gender -> Course -> Stroke -> Distance (pruning invalid events) -> Date range -> Age
fn divide(
    req: TopTimesRequest,
    by_course: bool,
    by_stroke: bool,
    by_distance: bool,
    by_date: bool,
    by_age: bool,
    by_zone: bool,
) -> Vec<TopTimesRequest> {
    let mut requests: Vec<TopTimesRequest> = Vec::new();

    if by_course && matches!(req.course, Course::All) {
        for course in COURSES {
            let mut r = req.clone();
            r.course = course;
            requests.push(r);
        }
        return requests;
    }

    if by_stroke && matches!(req.stroke, Stroke::All) {
        for stroke in IND_STROKES {
            let mut r = req.clone();
            r.stroke = stroke;
            requests.push(r);
        }
        return requests;
    }

    if by_distance && req.distance == 0 {
        for distance in DISTANCES {
            let mut r = req.clone();
            let event = SwimEvent {
                distance,
                stroke: r.stroke.clone(),
                course: r.course.clone(),
            };
            // FIXME: consider using a HashSet for faster validation
            if VALID_EVENTS.contains(&event) {
                r.distance = distance;
                requests.push(r);
            }
        }
        return requests;
    }

    // FIXME: this logic could be cleaner
    if by_date && req.from_date != req.to_date {
        let mut left = req.clone();
        let mut right = req.clone();
        let mid = (req.to_date - req.from_date).num_days() / 2;
        if mid == 0 {
            left.to_date = left.from_date;
            right.from_date = right.to_date;
        } else {
            left.to_date = req.from_date + Duration::days(mid - 1);
            right.from_date = req.from_date + Duration::days(mid);
            if right.from_date > right.to_date {
                right.from_date = right.to_date
            }
        }
        trace!(
            "mid={:?}, left_from={:?}, left_to={:?}, right_from={:?}, right_to={:?}",
            mid,
            left.from_date,
            left.to_date,
            right.from_date,
            right.to_date
        );
        requests.push(left);
        requests.push(right);
        return requests;
    }

    if by_age && req.start_age == Some(0) && req.end_age.is_none() {
        let mut r1 = req.clone();
        r1.start_age = Some(0);
        r1.end_age = Some(5);
        requests.push(r1);

        for i in 6u8..30u8 {
            let mut r = req.clone();
            r.start_age = Some(i);
            r.end_age = Some(i);
            requests.push(r);
        }

        let mut r2 = req.clone();
        r2.start_age = Some(30);
        r2.end_age = None;
        requests.push(r2);

        return requests;
    }

    if by_zone && matches!(req.zone, Zone::All) {
        for zone in ZONES {
            let mut r = req.clone();
            r.zone = zone;
            requests.push(r);
        }
        return requests;
    }

    requests
}

/// Generates a list of TopTimesRequests necessary to fetch all swimming times within a date range.
fn gen_requests_smart(from_date: NaiveDate, to_date: NaiveDate) -> Vec<TopTimesRequest> {
    let mut requests: Vec<TopTimesRequest> = Vec::new();

    let age_range = [
        (0, 7),
        (8, 8),
        (9, 9),
        (10, 10),
        (11, 11),
        (12, 12),
        (13, 13),
        (14, 14),
        (15, 15),
        (16, 16),
        (17, 17),
        (18, 18),
        (19, 19),
        (20, 20),
        (21, 21),
        (22, 22),
        (23, 23),
        (24, 24),
        (25, 51),
    ];

    let num_days = (to_date - from_date).num_days() as usize + 1;
    for d in from_date.iter_days().take(num_days) {
        for (start_age, end_age) in age_range {
            let actual_end_age = match end_age {
                51 => None,
                _ => Some(end_age as u8),
            };
            let r = TopTimesRequest {
                gender: Gender::Male,
                from_date: d,
                to_date: d,
                start_age: Some(start_age as u8),
                end_age: actual_end_age,
                max_results: 5000,
                ..TopTimesRequest::default()
            };

            let mut fr = r.clone();
            fr.stroke = Stroke::Freestyle;
            let mut bk = r.clone();
            bk.stroke = Stroke::Backstroke;
            let mut br = r.clone();
            br.stroke = Stroke::Breaststroke;
            let mut fl = r.clone();
            fl.stroke = Stroke::Butterfly;
            let mut im = r.clone();
            im.stroke = Stroke::IndividualMedley;

            // FIXME: add back in  400, 500, 800, 1000, 1500, 1650
            for dist in [50u16, 100, 200] {
                let mut fr_clone = fr.clone();
                fr_clone.distance = dist;
                requests.push(fr_clone)
            }
            for dist in [50u16, 100, 200] {
                let mut bk_clone = bk.clone();
                bk_clone.distance = dist;
                requests.push(bk_clone)
            }
            requests.push(br);
            requests.push(fl);
            requests.push(im);
        }
    }

    requests
}

async fn handle_signals(signals: Signals, tx: async_channel::Sender<TopTimesRequest>) {
    let mut signals = signals.fuse();
    while let Some(signal) = signals.next().await {
        match signal {
            SIGUSR1 => {
                let depth = tx.len();
                gauge!("mirror_request_queue_depth", depth as f64);
                info!("Request queue depth: {}", depth);
            }
            _ => unreachable!(),
        }
    }
}

pub fn debug() {
    let from_date = NaiveDate::from_ymd(2020, 1, 1);
    let to_date = NaiveDate::from_ymd(2020, 12, 31);
    let requests = gen_requests_smart(from_date, to_date);
    println!("Number of requests generated: {}", requests.len());
    for r in requests {
        println!("{}", r);
    }
}
