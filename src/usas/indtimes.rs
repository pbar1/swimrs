use std::collections::HashMap;

use chrono::{offset::Local, Duration, NaiveDate, NaiveDateTime};
use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::usas::model::*;

/// Errors that can be encountered during an Individual Times Search.
#[derive(Debug, Error)]
pub enum IndTimesError {
    #[error("unknown sanction status: {0}")]
    UnknownSanctionStatus(String),
    #[error("unknown stroke: {0}")]
    UnknownStroke(String),
    #[error("unknown course: {0}")]
    UnknownCourse(String),
    #[error("unable to build http client")]
    ClientBuild,
    #[error("unable to build regex")]
    RegexBuild,
    #[error("no times found")]
    NoTimes,
    #[error("unable to deserialize raw input")]
    DeserializeRaw,
    #[error("placeholder")]
    ParseDate,
    #[error("todo: implement error")]
    Todo,
}

/// Input for Individual Times Search.
#[derive(Debug)]
pub struct IndTimesRequest {
    pub first_name: String,
    pub last_name: String,
    pub from_date: NaiveDate,
    pub to_date: NaiveDate,
    pub distance: u16,
    pub stroke: Stroke,
    pub course: Course,
    pub start_age: Option<u8>,
    pub end_age: Option<u8>,
}

/// A swimming time entry from Individual Times Search.
#[derive(Debug, Serialize)]
pub struct IndTime {
    pub stroke: Stroke,
    pub course: Course,
    pub age: u8,
    pub swim_time: f64,
    pub alt_adj_time: f64,
    pub power_points: u16,
    pub standard: String,
    pub meet_name: String,
    pub lsc: String,
    pub club: String,
    pub swim_date: NaiveDate,
    pub person_clustered_id: String,
    pub meet_id: usize,
    pub time_id: usize,
    pub distance: u16,
    pub sanctioned: bool,
    pub relay: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct IndTimeRaw {
    stroke: String,
    course: String,
    age: u8,
    swim_time: String,
    alt_adj_time: String,
    power_points: u16,
    standard: String,
    meet_name: String,
    club: String,
    swim_date: String,
    event_sort_order: usize,
    time_id: usize,
    distance: u16,
    sanction_status: String,
    swim_time_for_sort: String,
    alt_adj_time_for_sort: String,

    #[serde(rename = "LSC")]
    lsc: String,

    #[serde(rename = "PersonClusteredID")]
    person_clustered_id: String,

    #[serde(rename = "EventID")]
    event_id: String,

    #[serde(rename = "MeetID")]
    meet_id: usize,
}

impl Default for IndTimesRequest {
    fn default() -> IndTimesRequest {
        let now = Local::now().naive_local().date();
        IndTimesRequest {
            first_name: String::from("*"),
            last_name: String::from("*"),
            from_date: now - Duration::days(365),
            to_date: now,
            distance: 0,
            stroke: Stroke::All,
            course: Course::All,
            start_age: None,
            end_age: None,
        }
    }
}

impl IndTimeRaw {
    fn to_individual_time(&self) -> Result<IndTime, IndTimesError> {
        let sanc = self.sanction_status.as_str();
        let sanctioned = match sanc {
            "Yes" => true,
            "No" => false,
            _ => return Err(IndTimesError::UnknownSanctionStatus(sanc.to_string())),
        };
        let relay = self.swim_time.contains('r');
        let event_split: Vec<&str> = self.stroke.split(' ').collect();
        let stroke = match event_split[1] {
            "FR" => Stroke::FR,
            "BK" => Stroke::BK,
            "BR" => Stroke::BR,
            "FL" => Stroke::FL,
            "IM" => Stroke::IM,
            "FR-R" => Stroke::FR_R,
            "MED-R" => Stroke::MED_R,
            _ => return Err(IndTimesError::UnknownStroke(event_split[1].to_string())),
        };
        let crs = self.course.as_str();
        let course = match crs {
            "LCM" => Course::LCM,
            "SCM" => Course::SCM,
            "SCY" => Course::SCY,
            _ => return Err(IndTimesError::UnknownCourse(crs.to_string())),
        };

        Ok(IndTime {
            stroke,
            course,
            age: self.age,
            swim_time: parse_seconds(self.swim_time_for_sort.as_str()),
            alt_adj_time: parse_seconds(self.alt_adj_time_for_sort.as_str()),
            power_points: self.power_points,
            standard: self.standard.clone(),
            meet_name: self.meet_name.clone(),
            lsc: self.lsc.clone(),
            club: self.club.clone(),
            swim_date: parse_date(self.swim_date.as_str()).unwrap(),
            person_clustered_id: self.person_clustered_id.clone(),
            meet_id: self.meet_id,
            time_id: self.time_id,
            distance: self.distance,
            sanctioned,
            relay,
        })
    }
}

pub async fn get_times(req: IndTimesRequest) -> Result<Vec<IndTime>, IndTimesError> {
    let resp = fetch_html(req).await?;
    parse(resp)
}

fn form_body(req: IndTimesRequest) -> HashMap<String, String> {
    let start_age = match req.start_age {
        Some(age) => age.to_string(),
        None => String::from("All"),
    };
    let end_age = match req.end_age {
        Some(age) => age.to_string(),
        None => String::from("All"),
    };
    let from_date = req.from_date.format("%-m/%-d/%Y").to_string();
    let to_date = req.to_date.format("%-m/%-d/%Y").to_string();
    let distance_id = req.distance.to_string();
    let stroke_id = (req.stroke as u8).to_string();
    let course_id = (req.course as u8).to_string();

    let mut params = HashMap::new();
    params.insert(
        String::from("DivId"),
        String::from("Times_TimesSearchDetail_Index_Div-1"),
    );
    params.insert(String::from("FirstName"), req.first_name);
    params.insert(String::from("LastName"), req.last_name);
    params.insert(String::from("PersonId"), String::from(""));
    params.insert(String::from("FromDate"), from_date);
    params.insert(String::from("ToDate"), to_date);
    params.insert(String::from("DateRangeId"), String::from("0"));
    params.insert(String::from("DistanceId"), distance_id);
    params.insert(String::from("StrokeId"), stroke_id);
    params.insert(String::from("CourseId"), course_id);
    params.insert(String::from("StartAge"), start_age);
    params.insert(String::from("EndAge"), end_age);
    params.insert(String::from("SelectedAgeFilter"), String::from("All"));
    params.insert(String::from("SortPeopleBy"), String::from("Name"));
    params.insert(String::from("SortTimesBy"), String::from("EventSortOrder"));

    params
}

async fn fetch_html(req: IndTimesRequest) -> Result<String, IndTimesError> {
    let client = reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .map_err(|e| IndTimesError::ClientBuild)?;
    let params = form_body(req);

    // Fetch the referring page to populate the cookie jar, which seems to be necessary
    client
        .get("https://www.usaswimming.org/times/individual-times-search")
        .send()
        .await
        .map_err(|e| IndTimesError::Todo)?;

    Ok(client
        .post("https://www.usaswimming.org/api/Times_TimesSearchDetail/ListTimes")
        .form(&params)
        .send()
        .await
        .map_err(|e| IndTimesError::Todo)?
        .text()
        .await
        .map_err(|e| IndTimesError::Todo)?)
}

fn parse(resp_html: String) -> Result<Vec<IndTime>, IndTimesError> {
    // TODO: check for errors in response
    let re = Regex::new(r"data: (\[.*])").map_err(|e| IndTimesError::RegexBuild)?;
    let caps = re.captures(resp_html.as_str()).unwrap();
    let output = caps.get(1).map_or("", |m| m.as_str());
    let raw_data: Vec<IndTimeRaw> =
        serde_json::from_str(output).map_err(|e| IndTimesError::DeserializeRaw)?;
    let data: Result<Vec<IndTime>, IndTimesError> =
        raw_data.iter().map(|x| x.to_individual_time()).collect();
    data
}

fn parse_seconds(swim_time: &str) -> f64 {
    let split: Vec<&str> = swim_time.split(':').collect();
    let minutes: f64 = split[0].parse().unwrap();
    let seconds: f64 = split[1].parse().unwrap();
    60f64 * minutes + seconds
}

fn parse_date(swim_date: &str) -> Result<NaiveDate, IndTimesError> {
    let seconds = swim_date
        .replace("/Date(", "")
        .replace(")/", "")
        .parse::<i64>()
        .map_err(|e| IndTimesError::ParseDate)?
        / 1000;
    let dt = NaiveDateTime::from_timestamp(seconds, 0).date();
    Ok(dt)
}

// mod tests {
//     use super::*
//
//
// }
