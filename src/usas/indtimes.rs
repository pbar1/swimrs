use std::{collections::HashMap, convert::TryFrom};

use chrono::{offset::Local, Duration, NaiveDate, NaiveDateTime};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::usas::model::{Course, Gender, Stroke, SwimError, TimeType, Zone, LSC};

/// Input for Individual Times Search.
#[derive(Debug)]
pub struct IndTimesRequest {
    /// First name of the swimmer to search for
    pub first_name: String,

    /// Last name of the swimmer to search for
    pub last_name: String,

    /// Starting date in the search range
    pub from_date: NaiveDate,

    /// Ending date in the search range
    pub to_date: NaiveDate,

    /// Swimming event distance to limit results to
    pub distance: u16,

    /// Swimming stroke to limit results to
    pub stroke: Stroke,

    /// Swimming course to limit results to
    pub course: Course,

    /// Starting age in the search range
    pub start_age: Option<u8>,

    /// Ending age in the search range
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

impl TryFrom<&IndTimeRaw> for IndTime {
    type Error = SwimError;

    fn try_from(raw: &IndTimeRaw) -> Result<Self, Self::Error> {
        let event_split: Vec<&str> = raw.stroke.split(' ').collect();
        let stroke = match event_split[1] {
            "FR" => Stroke::Freestyle,
            "BK" => Stroke::Backstroke,
            "BR" => Stroke::Breaststroke,
            "FL" => Stroke::Butterfly,
            "IM" => Stroke::IndividualMedley,
            "FR-R" => Stroke::FreestyleRelay,
            "MED-R" => Stroke::MedleyRelay,
            _ => return Err(SwimError::UnknownStroke(event_split[1].to_string())),
        };
        let course = match raw.course.as_str() {
            "LCM" => Course::LCM,
            "SCM" => Course::SCM,
            "SCY" => Course::SCY,
            _ => return Err(SwimError::UnknownCourse(raw.course.clone())),
        };
        let sanctioned = match raw.sanction_status.as_str() {
            "Yes" => true,
            "No" => false,
            _ => {
                return Err(SwimError::UnknownSanctionStatus(
                    raw.sanction_status.clone(),
                ))
            }
        };
        let relay = raw.swim_time.contains('r');

        Ok(IndTime {
            stroke,
            course,
            age: raw.age,
            swim_time: parse_seconds(raw.swim_time_for_sort.as_str()),
            alt_adj_time: parse_seconds(raw.alt_adj_time_for_sort.as_str()),
            power_points: raw.power_points,
            standard: raw.standard.clone(),
            meet_name: raw.meet_name.clone(),
            lsc: raw.lsc.clone(),
            club: raw.club.clone(),
            swim_date: parse_date(raw.swim_date.as_str()).unwrap(),
            person_clustered_id: raw.person_clustered_id.clone(),
            meet_id: raw.meet_id,
            time_id: raw.time_id,
            distance: raw.distance,
            sanctioned,
            relay,
        })
    }
}

pub async fn get_times(req: IndTimesRequest) -> Result<Vec<IndTime>, SwimError> {
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

async fn fetch_html(req: IndTimesRequest) -> Result<String, SwimError> {
    let client = reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .map_err(|_e| SwimError::ClientBuild)?;
    let params = form_body(req);

    // Fetch the referring page to populate the cookie jar, which seems to be necessary
    client
        .get("https://www.usaswimming.org/times/individual-times-search")
        .send()
        .await
        .map_err(|_e| SwimError::Todo)?;

    Ok(client
        .post("https://www.usaswimming.org/api/Times_TimesSearchDetail/ListTimes")
        .form(&params)
        .send()
        .await
        .map_err(|_e| SwimError::Todo)?
        .text()
        .await
        .map_err(|_e| SwimError::Todo)?)
}

fn parse(resp_html: String) -> Result<Vec<IndTime>, SwimError> {
    // FIXME: check for errors in response
    let re = Regex::new(r"data: (\[.*])").map_err(|_e| SwimError::RegexBuild)?;
    let caps = re.captures(resp_html.as_str()).unwrap();
    let output = caps.get(1).map_or("", |m| m.as_str());
    let raw_data: Vec<IndTimeRaw> =
        serde_json::from_str(output).map_err(|_e| SwimError::DeserializeRaw)?;
    let data: Result<Vec<IndTime>, SwimError> = raw_data.iter().map(IndTime::try_from).collect();
    data
}

fn parse_seconds(swim_time: &str) -> f64 {
    let split: Vec<&str> = swim_time.split(':').collect();
    let minutes: f64 = split[0].parse().unwrap();
    let seconds: f64 = split[1].parse().unwrap();
    60f64 * minutes + seconds
}

fn parse_date(swim_date: &str) -> Result<NaiveDate, SwimError> {
    let seconds = swim_date
        .replace("/Date(", "")
        .replace(")/", "")
        .parse::<i64>()
        .map_err(|_e| SwimError::ParseDate)?
        / 1000;
    let dt = NaiveDateTime::from_timestamp(seconds, 0).date();
    Ok(dt)
}
