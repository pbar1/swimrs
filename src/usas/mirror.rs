use std::{collections::VecDeque, error::Error, io::BufRead, process::Stdio};

use chrono::{Duration, NaiveDate};
use futures::{future::join_all, stream, StreamExt};
use governor::{Quota, RateLimiter};
use log::{debug, error, info, trace};
use nonzero_ext::nonzero;
use rand::seq::SliceRandom;
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

// // TODO make sure to handle Gender in the model
// https://stackoverflow.com/questions/51044467/how-can-i-perform-parallel-asynchronous-http-get-requests-with-reqwest
pub async fn mirror(concurrency: usize, dry_run: bool) -> Result<(), Box<dyn Error>> {
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
                    debug!("Started Tor proxy {}", i);
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
        let mut lim = RateLimiter::direct(Quota::per_second(nonzero!(1u32)));
        let rx2 = rx.clone();
        let proxy_addr = format!("socks5://127.0.0.1:{}", 53000 + i);
        let client = toptimes::TopTimesClient::new(proxy_addr.as_str()).unwrap();
        let handle = tokio::spawn(async move {
            client.populate_cookies().await.unwrap();
            loop {
                select! {
                    Ok(r) = rx2.recv() => {
                        let file = format!("results/{}/result.csv", r);
                        if std::path::Path::new(file.as_str()).exists() {
                            debug!("Results file already exists: {}", file);
                            continue;
                        }
                        debug!("Making request on client {}: {}", i, r);
                        let client2 = client.clone();
                        make_request(client2, r).await;
                        lim.until_ready().await;
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
    let root_req = toptimes::TopTimesRequest {
        gender: Gender::Male,
        distance: 0,
        stroke: Stroke::All,
        course: Course::All,
        from_date: NaiveDate::from_ymd(2021, 1, 1),
        to_date: NaiveDate::from_ymd(2021, 5, 30),
        start_age: Some(0),
        end_age: None,
        max_results: 5000,
        ..toptimes::TopTimesRequest::default()
    };
    let mut requests = atomize(root_req, true, true, true, true, true, false);
    debug!("Generated {} total requests", requests.len());
    let mut rng = rand::thread_rng();
    requests.shuffle(&mut rng);
    for r in requests {
        tx.send(r).await?;
    }
    tx.close();

    // TODO: Start HTTP server to expose channel depth

    // Begin processing all of the requests
    join_all(client_handles).await;

    Ok(())
}

async fn make_request(client: TopTimesClient, req: TopTimesRequest) {
    let dir = format!("results/{}/", req);
    tokio::fs::create_dir_all(dir.clone()).await.unwrap();
    let req2 = req.clone();
    match client.search(req).await {
        Ok(times) => {
            info!("Found {} times for request: {}", times.len(), req2);
            let mut wtr = csv::Writer::from_path(dir + "result.csv").unwrap();
            for t in times {
                wtr.serialize(t).unwrap();
            }
            wtr.flush().unwrap();
        }
        Err(e) => {
            error!("Error executing search request: req={}, err={}", req2, e);
            tokio::fs::File::create(dir + "error.txt").await.unwrap();
        }
    }
}

/// Divides the given TopTimesRequest into the smallest possible chunks that
/// obey the given parameters.
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
