use std::{collections::HashMap, error::Error};

use chrono::{offset::Local, Duration, NaiveDate};
use regex::Regex;
use serde_json::json;

use self::model::{Course, Gender, Stroke, TimeType, Zone, LSC};

mod indtimes;
pub mod model;
mod toptimes;

const BASE_URL: &str = "https://www.usaswimming.org";

pub struct USASClient {
    http_client: reqwest::Client,
}

impl USASClient {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        let http_client = reqwest::Client::builder().cookie_store(true).build()?;
        // http_client
        // .get(format!(
        //     "{}{}",
        //     BASE_URL, "/times/popular-resources/event-rank-search"
        // ))
        // .send()
        // .await?;
        Ok(USASClient { http_client })
    }
}

pub async fn example_individual_times() -> Result<(), Box<dyn Error>> {
    let usas_client = USASClient::new().await?;

    let req = indtimes::IndTimesRequest {
        first_name: String::from("Ryan"),
        last_name: String::from("Lochte"),
        from_date: NaiveDate::from_ymd(2016, 1, 1),
        to_date: NaiveDate::from_ymd(2016, 12, 30),
        ..indtimes::IndTimesRequest::default()
    };

    let output = indtimes::get_times(req).await?;

    println!("{:#?}", output);

    Ok(())
}

// pub async fn example_top_times() -> Result<(), Box<dyn Error>> {
//     let usas_client = USASClient::new().await?;
//
//     let top_times_req = TopTimesRequest {
//         gender: Gender::Male,
//         distance: 50,
//         stroke: Stroke::FR,
//         course: Course::LCM,
//         from_date: NaiveDate::from_ymd(2021, 01, 01),
//         to_date: Local::now().naive_local().date(),
//         start_age: Some(20),
//         end_age: Some(25),
//         zone: Zone::All,
//         lscs: vec![LSC::All],
//         time_type: TimeType::Individual,
//         members_only: false,
//         best_only: false,
//         max_results: 5000,
//     };
//
//     let output = usas_client.top_times_raw(top_times_req).await?;
//
//     println!("{}", output);
//
//     Ok(())
// }
