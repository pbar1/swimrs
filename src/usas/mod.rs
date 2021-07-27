use std::{error::Error, fs};

use chrono::NaiveDate;
use futures::{stream, StreamExt};
use log::{error, info};

use crate::usas::model::{Course, Gender, Stroke, VALID_EVENTS};

pub mod indtimes;
pub mod model;
pub mod toptimes;

pub async fn example_individual_times() -> Result<(), Box<dyn Error>> {
    let client = indtimes::IndTimesClient::new()?;
    client.populate_cookies().await?;

    let req = indtimes::IndTimesRequest {
        first_name: String::from("Caeleb"),
        last_name: String::from("Dressel"),
        from_date: NaiveDate::from_ymd(2019, 7, 26),
        to_date: NaiveDate::from_ymd(2019, 7, 26),
        distance: 100,
        stroke: Stroke::Butterfly,
        course: Course::LCM,
        ..indtimes::IndTimesRequest::default()
    };
    let output = client.search(req).await?;
    println!("{:#?}", output);
    Ok(())
}

pub async fn example_top_times() -> Result<(), Box<dyn Error>> {
    let client = toptimes::TopTimesClient::new()?;
    client.populate_cookies().await?;

    let req = toptimes::TopTimesRequest {
        gender: Gender::Male,
        distance: 200,
        stroke: Stroke::Freestyle,
        course: Course::LCM,
        from_date: NaiveDate::from_ymd(2008, 1, 1),
        to_date: NaiveDate::from_ymd(2008, 12, 30),
        max_results: 10,
        ..toptimes::TopTimesRequest::default()
    };
    let output = client.search(req).await?;

    let mut wtr = csv::Writer::from_writer(std::io::stdout());
    for rec in output {
        wtr.serialize(rec)?;
    }
    // wtr.serialize(output);
    wtr.flush()?;

    // println!("{:#?}", output);
    Ok(())
}

// https://stackoverflow.com/questions/51044467/how-can-i-perform-parallel-asynchronous-http-get-requests-with-reqwest
pub async fn mirror(concurrency: usize) -> Result<(), Box<dyn Error>> {
    let client = toptimes::TopTimesClient::new()?;
    client.populate_cookies().await?;

    // TODO make sure to handle Gender in the model
    let buffered = stream::iter(VALID_EVENTS)
        .map(|event| {
            info!("event: {:?}", event);
            let client = &client;
            let req = toptimes::TopTimesRequest {
                gender: Gender::Male,
                distance: event.distance,
                stroke: event.stroke,
                course: event.course,
                from_date: NaiveDate::from_ymd(2008, 4, 5),
                to_date: NaiveDate::from_ymd(2008, 4, 12),
                ..toptimes::TopTimesRequest::default()
            };
            async move { client.search(req).await }
        })
        .buffer_unordered(concurrency);

    buffered
        .for_each(|b| async {
            match b {
                Ok(b) => println!("Found {} results", b.len()),
                Err(e) => error!("Error searching for times: {:?}", e),
            }
        })
        .await;

    Ok(())
}
