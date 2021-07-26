use std::{collections::HashMap, convert::TryFrom, error::Error, str::FromStr};

use chrono::{offset::Local, Duration, NaiveDate, NaiveDateTime};
use log::debug;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::usas::model::{Course, Stroke, SwimEvent, SwimTime};

pub struct IndTimesClient {
    client: Client,
}

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
    pub swim_time: f32,
    pub alt_adj_time: f32,
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

impl TryFrom<IndTimeRaw> for IndTime {
    type Error = Box<dyn Error>;

    fn try_from(raw: IndTimeRaw) -> Result<Self, Self::Error> {
        debug!("Converting to IndTime: {:?}", raw);

        let swim_event = SwimEvent::from_str(format!("{} {}", raw.stroke, raw.course).as_str())?;
        let swim_time = SwimTime::from_str(raw.swim_time.as_str())?;
        let alt_adj_swim_time = SwimTime::from_str(raw.alt_adj_time.as_str())?;
        let sanctioned = raw.sanction_status == "Yes";
        let swim_date = parse_date(raw.swim_date.as_str())?;

        Ok(IndTime {
            stroke: swim_event.stroke,
            course: swim_event.course,
            age: raw.age,
            swim_time: swim_time.seconds,
            alt_adj_time: alt_adj_swim_time.seconds,
            power_points: raw.power_points,
            standard: raw.standard,
            meet_name: raw.meet_name,
            lsc: raw.lsc,
            club: raw.club,
            swim_date,
            person_clustered_id: raw.person_clustered_id,
            meet_id: raw.meet_id,
            time_id: raw.time_id,
            distance: raw.distance,
            sanctioned,
            relay: swim_time.relay,
        })
    }
}

impl From<IndTimesRequest> for HashMap<&'static str, String> {
    fn from(req: IndTimesRequest) -> Self {
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
        params.insert("DivId", String::from("Times_TimesSearchDetail_Index_Div-1"));
        params.insert("FirstName", req.first_name);
        params.insert("LastName", req.last_name);
        params.insert("PersonId", String::from(""));
        params.insert("FromDate", from_date);
        params.insert("ToDate", to_date);
        params.insert("DateRangeId", String::from("0"));
        params.insert("DistanceId", distance_id);
        params.insert("StrokeId", stroke_id);
        params.insert("CourseId", course_id);
        params.insert("StartAge", start_age);
        params.insert("EndAge", end_age);
        params.insert("SelectedAgeFilter", String::from("All"));
        params.insert("SortPeopleBy", String::from("Name"));
        params.insert("SortTimesBy", String::from("EventSortOrder"));

        params
    }
}

impl IndTimesClient {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let client = Client::builder().cookie_store(true).build()?;
        Ok(IndTimesClient { client })
    }

    pub async fn populate_cookies(&self) -> Result<(), Box<dyn Error>> {
        self.client
            .get("https://www.usaswimming.org/times/individual-times-search")
            .send()
            .await?;
        Ok(())
    }

    pub async fn search(&self, req: IndTimesRequest) -> Result<Vec<IndTime>, Box<dyn Error>> {
        let resp = self.fetch_raw(req).await?;
        parse(resp)
    }

    async fn fetch_raw(&self, req: IndTimesRequest) -> Result<String, Box<dyn Error>> {
        let request_body = HashMap::from(req);
        Ok(self
            .client
            .post("https://www.usaswimming.org/api/Times_TimesSearchDetail/ListTimes")
            .form(&request_body)
            .send()
            .await?
            .text()
            .await?)
    }
}

// FIXME implement from trait
// FIXME: check for errors in response
fn parse(resp_html: String) -> Result<Vec<IndTime>, Box<dyn Error>> {
    let re = Regex::new(r"data: (\[.*])")?;
    let caps = re.captures(resp_html.as_str()).unwrap();
    let output = caps.get(1).map_or("", |m| m.as_str());
    let raw_data: Vec<IndTimeRaw> = serde_json::from_str(output)?;
    let data: Result<Vec<IndTime>, Box<dyn Error>> =
        raw_data.into_iter().map(IndTime::try_from).collect();
    data
}

fn parse_date(swim_date: &str) -> Result<NaiveDate, Box<dyn Error>> {
    let seconds = swim_date
        .replace("/Date(", "")
        .replace(")/", "")
        .parse::<i64>()?
        / 1000;
    let dt = NaiveDateTime::from_timestamp(seconds, 0).date();
    Ok(dt)
}
