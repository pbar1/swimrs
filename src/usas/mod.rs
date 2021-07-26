use std::error::Error;

use chrono::NaiveDate;

use crate::usas::model::{Course, Gender, Stroke};

pub mod indtimes;
pub mod model;
pub mod toptimes;

pub async fn example_individual_times() -> Result<(), Box<dyn Error>> {
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
    let output = indtimes::get_times(req).await?;
    println!("{:#?}", output);
    Ok(())
}

pub async fn example_top_times() -> Result<(), Box<dyn Error>> {
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
    let output = toptimes::search(req).await?;

    let mut wtr = csv::Writer::from_writer(std::io::stdout());
    for rec in output {
        wtr.serialize(rec);
    }
    // wtr.serialize(output);
    wtr.flush();

    // println!("{:#?}", output);
    Ok(())
}
