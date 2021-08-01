use std::error::Error;

use chrono::{Duration, Local, NaiveDate};
use log::{debug, error, info, trace};
use stopwatch::Stopwatch;
use tokio::sync::mpsc;

use crate::usas::{
    model::{Course, Gender, Stroke, SwimEvent, COURSES, DISTANCES, IND_STROKES, VALID_EVENTS},
    toptimes,
    toptimes::{TopTime, TopTimesClient, TopTimesRequest},
};

// // TODO make sure to handle Gender in the model
// https://stackoverflow.com/questions/51044467/how-can-i-perform-parallel-asynchronous-http-get-requests-with-reqwest
pub async fn mirror(concurrency: usize, dry_run: bool) -> Result<(), Box<dyn Error>> {
    let client = toptimes::TopTimesClient::new()?;
    client.populate_cookies().await?;

    // Seed the request channel with its initial value
    let (tx, mut rx) = mpsc::channel(concurrency);
    let root_req = toptimes::TopTimesRequest {
        gender: Gender::Male,
        from_date: NaiveDate::from_ymd(2021, 1, 1), // TODO widen this range
        to_date: Local::now().naive_local().date() - Duration::weeks(3),
        max_results: 100_000,
        ..toptimes::TopTimesRequest::default()
    };
    tx.send(root_req).await?;

    // Receive from the request pipeline on a separate task continuously, and produce tasks to execute them
    let (task_tx, mut task_rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        debug!("Spawning generator thread");

        while let Some(req) = rx.recv().await {
            trace!("Received request from channel: {:?}", req);

            let c = client.clone();
            let tx2 = tx.clone();

            let handle = tokio::spawn(async move {
                let req2 = req.clone();

                let sw = Stopwatch::start_new();
                let res = make_request(c, req).await;
                info!("Request took {} ms: {:?}", sw.elapsed_ms(), req2);

                match res {
                    Some(r) => Some((req2, r)),
                    None => {
                        for r in divide(req2) {
                            tx2.send(r).await.unwrap();
                        }
                        None
                    }
                }
            });

            task_tx.send(handle).unwrap();
        }
    });

    // Drain the task channel
    while let Some(handle) = task_rx.recv().await {
        match handle.await {
            Ok(o) => match o {
                Some((req, res)) => {
                    debug!("Found {} times for request: {:?}", res.len(), req);
                    let dir = format!(
                        "results/{:?}/{:?}/{:?}/{}/{}_{}/",
                        req.gender,
                        req.course,
                        req.stroke,
                        req.distance,
                        req.from_date,
                        req.to_date
                    );
                    std::fs::create_dir_all(dir.clone()).unwrap();
                    let mut wtr = csv::Writer::from_path(dir + "result.csv").unwrap();
                    for record in res {
                        wtr.serialize(record).unwrap();
                    }
                    wtr.flush().unwrap();
                }
                None => debug!("No times found"),
            },
            Err(e) => error!("error awaiting task: {}", e),
        }
    }

    Ok(())
}

async fn make_request(client: TopTimesClient, req: TopTimesRequest) -> Option<Vec<TopTime>> {
    // let client = &client;
    match client.search(req).await {
        Ok(r) => Some(r),
        Err(_) => None,
    }
}

/// Divides the given TopTimesRequest into smaller chunks.
/// Gender -> Course -> Stroke -> Distance (pruning invalid events) -> Date range -> Age
fn divide(req: TopTimesRequest) -> Vec<TopTimesRequest> {
    let mut requests: Vec<TopTimesRequest> = Vec::new();

    if matches!(req.course, Course::All) {
        for course in COURSES {
            let mut r = req.clone();
            r.course = course;
            requests.push(r);
        }
        return requests;
    }

    if matches!(req.stroke, Stroke::All) {
        for stroke in IND_STROKES {
            let mut r = req.clone();
            r.stroke = stroke;
            requests.push(r);
        }
        return requests;
    }

    if req.distance == 0 {
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
    if req.from_date != req.to_date {
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

    // FIXME: this logic could be cleaner
    let start_age = req.start_age.unwrap_or(0);
    let end_age = req.end_age.unwrap_or(51);
    if start_age != end_age {
        let mut left = req.clone();
        let mut right = req.clone();
        let mid = (end_age - start_age) / 2;

        if mid == 0 {
            left.end_age = left.start_age;
            right.start_age = right.end_age;
        } else {
            left.end_age = Some(mid - 1);
            right.start_age = Some(mid);
        }

        if left.start_age == Some(51) {
            left.start_age = None
        }
        if left.end_age == Some(51) {
            left.end_age = None
        }
        if right.start_age == Some(51) {
            right.start_age = None
        }
        if right.end_age == Some(51) {
            right.end_age = None
        }

        requests.push(left);
        requests.push(right);
        return requests;
    }

    requests
}
