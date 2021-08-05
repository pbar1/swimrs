use std::{
    convert::TryFrom,
    fmt::{Display, Formatter},
    str::FromStr,
};

use anyhow::{bail, Error};
use chrono::{offset::Local, Duration, NaiveDate};
use log::debug;
use metrics::{decrement_gauge, gauge, histogram, increment_counter, increment_gauge};
use reqwest::{Client, Proxy};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use stopwatch::Stopwatch;

use crate::usas::model::{Course, Gender, Stroke, SwimEvent, SwimTime, TimeType, Zone, LSC};

const MAIN_URL: &str = "https://www.usaswimming.org/times/popular-resources/event-rank-search";
const KEY_URL: &str =
    "https://www.usaswimming.org/times/popular-resources/event-rank-search/CsvTimes";
const REPORT_URL: &str = "https://www.usaswimming.org/api/Reports_ReportViewer/GetReport";

#[derive(Debug, Clone)]
pub struct TopTimesClient {
    client: Client,
}

/// Input for Top Times / Event Rank Search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopTimesRequest {
    /// Gender to search for.
    pub gender: Gender,

    /// Swimming event distance to limit results to. A value of `0` signifies "all distances".
    pub distance: u16,

    /// Swimming stroke to limit results to.
    pub stroke: Stroke,

    /// Swimming course to limit results to.
    pub course: Course,

    /// Starting date in the search range.
    pub from_date: NaiveDate,

    /// /// Ending date in the search range.
    pub to_date: NaiveDate,

    /// Starting age in the search range.
    pub start_age: Option<u8>,

    /// Ending age in the search range.
    pub end_age: Option<u8>,

    /// USA Swimming Zone to limit results to.
    pub zone: Zone,

    /// List of USA Swimming Local Swimming Committees (LSCs) to limit results to.
    pub lscs: Vec<LSC>,

    /// Time type to search for.
    pub time_type: TimeType,

    /// Limit results to include only USA Swimming members.
    pub members_only: bool,

    /// Limit results to include only the best time for each swimmer in the range.
    pub best_only: bool,

    /// Limit results to this many entries.
    pub max_results: u32,
}

#[derive(Debug, Serialize)]
pub struct TopTime {
    pub rank: usize,
    pub full_name: String,
    pub time_id: String,
    pub distance: u16,
    pub stroke: Stroke,
    pub course: Course,
    pub age: u8,
    pub swim_time_seconds: f32,
    pub alt_adj_swim_time_seconds: f32,
    pub standard_name: String,
    pub meet_name: String,
    pub swim_date: NaiveDate,
    pub club_name: String,
    pub lsc_id: String,
    pub foreign: bool,
    pub hytek_power_points: u16,
    pub sanctioned: bool,
    pub relay: bool,
}

// ="result_rank",="full_name",="distance",="time_id",="event_desc",="swimmer_age",="swim_time_formatted",="alt_adj_swim_time_formatted",="standard_name",="meet_name",="swim_date",="club_name",="lsc_id",="foreign_yesno",="hytek_power_points",="event_id",="sanction_status"
// ="1","Hancock, Rick",="100",="1077981",="100 BK SCY",="10",="1:01.35",="1:01.35",="""AAAA""",="1996 US NAG Records B",="1/2/1996",="Unattached",="US",="",="1004",="12",="Yes"
#[derive(Debug, Deserialize)]
struct TopTimeRaw {
    result_rank: usize,
    full_name: String,
    distance: u16,
    time_id: String,
    event_desc: String,
    swimmer_age: u8,
    swim_time_formatted: String,
    alt_adj_swim_time_formatted: String,
    standard_name: String,
    meet_name: String,
    swim_date: String,
    club_name: String,
    lsc_id: String,
    foreign_yesno: String,
    hytek_power_points: u16,
    event_id: String,
    sanction_status: String,
}

impl TopTimesClient {
    pub fn new(proxy_addr: &str) -> Result<Self, Error> {
        let proxy = Proxy::all(proxy_addr)?;
        let client = Client::builder().cookie_store(true).proxy(proxy).build()?;
        Ok(TopTimesClient { client })
    }

    pub async fn populate_cookies(&self) -> Result<(), Error> {
        let mut sw = Stopwatch::start_new();
        self.client.get(MAIN_URL).send().await?.error_for_status()?;
        sw.stop();
        histogram!("usas_toptimes_request_duration_seconds", sw.elapsed(), "endpoint" => MAIN_URL);
        increment_counter!("usas_toptimes_requests", "endpoint" => MAIN_URL);
        Ok(())
    }

    pub async fn search(&self, req: TopTimesRequest) -> Result<Vec<TopTime>, Error> {
        let data_csv = self.fetch_raw(req).await?;
        let mut rdr = csv::ReaderBuilder::new().from_reader(data_csv.as_bytes());

        // FIXME: turn this into a chained map
        let mut data_raw: Vec<TopTimeRaw> = vec![];
        for r in rdr.deserialize() {
            let rec: TopTimeRaw = r?;
            data_raw.push(rec);
        }

        let data: Result<Vec<TopTime>, Error> =
            data_raw.into_iter().map(TopTime::try_from).collect();
        data
    }

    async fn fetch_raw(&self, req: TopTimesRequest) -> Result<String, Error> {
        let body = Value::from(req);

        let mut key_sw = Stopwatch::start_new();
        let key = self
            .client
            .post(KEY_URL)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        key_sw.stop();
        histogram!("usas_toptimes_request_duration_seconds", key_sw.elapsed(), "endpoint" => KEY_URL);
        increment_counter!("usas_toptimes_requests", "endpoint" => KEY_URL);

        // key should be an 89-character base64 string ending in "=="
        if key.len() != 89 && !key.ends_with("==") {
            bail!("Expected Top Times CSV report key, found: {}", key)
        }

        let mut report_sw = Stopwatch::start_new();
        let report = self
            .client
            .get(REPORT_URL)
            .query(&[
                ("Key", key),
                ("Format", String::from("Csv")),
                ("IsFileDownload", String::from("false")),
            ])
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?
            .replace("=\"", "\"");
        report_sw.stop();
        histogram!("usas_toptimes_request_duration_seconds", report_sw.elapsed(), "endpoint" => REPORT_URL);
        increment_counter!("usas_toptimes_requests", "endpoint" => REPORT_URL);

        match report.contains("Please rerun the report") {
            true => bail!("Failed to fetch Top Times report"),
            false => Ok(report),
        }
    }
}

impl Default for TopTimesRequest {
    /// Creates a default Top Times / Event Rank Search request. Date range is
    /// the past week up to the current date. Includes all distances, strokes,
    /// courses, ages, Zones, LSCs; both members and non-members; all times for
    /// each swimmer. Caps results to a total of 5000.
    fn default() -> TopTimesRequest {
        TopTimesRequest {
            gender: Gender::Mixed,
            distance: 0,
            stroke: Stroke::All,
            course: Course::All,
            from_date: Local::now().naive_local().date() - Duration::weeks(1),
            to_date: Local::now().naive_local().date(),
            start_age: None,
            end_age: None,
            zone: Zone::All,
            lscs: vec![LSC::All],
            time_type: TimeType::Individual,
            members_only: false,
            best_only: false,
            max_results: 5000,
        }
    }
}

impl Display for TopTimesRequest {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let start_age = match self.start_age {
            Some(i) => format!("{}", i),
            None => String::from("All"),
        };
        let end_age = match self.end_age {
            Some(i) => format!("{}", i),
            None => String::from("All"),
        };
        write!(
            f,
            "{:?}/{:?}/{:?}/{}/{}_{}/{}_{}/{:?}",
            self.gender,
            self.course,
            self.stroke,
            self.distance,
            self.from_date,
            self.to_date,
            start_age,
            end_age,
            self.zone,
        )
    }
}

impl TryFrom<TopTimeRaw> for TopTime {
    type Error = anyhow::Error;

    fn try_from(value: TopTimeRaw) -> Result<Self, Self::Error> {
        debug!("Converting to TopTime: {:?}", value);

        let swim_event = SwimEvent::from_str(value.event_desc.as_str())?;
        let swim_time = SwimTime::from_str(value.swim_time_formatted.as_str())?;
        let alt_adj_swim_time = SwimTime::from_str(value.alt_adj_swim_time_formatted.as_str())?;
        let swim_date = NaiveDate::parse_from_str(value.swim_date.as_str(), "%-m/%-d/%Y")?;
        let sanctioned = value.sanction_status == "Yes";
        let foreign = value.foreign_yesno == "Yes";

        Ok(TopTime {
            rank: value.result_rank,
            full_name: value.full_name,
            time_id: value.time_id,
            distance: swim_event.distance,
            stroke: swim_event.stroke,
            course: swim_event.course,
            age: value.swimmer_age,
            swim_time_seconds: swim_time.seconds,
            alt_adj_swim_time_seconds: alt_adj_swim_time.seconds,
            standard_name: value.standard_name,
            meet_name: value.meet_name,
            swim_date,
            club_name: value.club_name,
            lsc_id: value.lsc_id,
            foreign,
            hytek_power_points: value.hytek_power_points,
            sanctioned,
            relay: swim_time.relay,
        })
    }
}

impl From<TopTimesRequest> for Value {
    fn from(req: TopTimesRequest) -> Self {
        debug!("Converting to Value: {:?}", req);

        let start_age = match req.start_age {
            Some(age) => age.to_string(),
            None => String::from("All"),
        };
        let end_age = match req.end_age {
            Some(age) => age.to_string(),
            None => String::from("All"),
        };
        let members_only = match req.members_only {
            true => "Yes",
            false => "No",
        };
        let best_only = match req.best_only {
            true => "Best",
            false => "All",
        };
        let from_date = req.from_date.format("%-m/%-d/%Y").to_string();
        let to_date = req.to_date.format("%-m/%-d/%Y").to_string();
        let value = json!({
            "DivId": "Times_TimesSearchTopTimesEventRankSearch_Index_Div-1",  // constant value
            "DateRangeId": "0",  // set to 0 to disable preset date range and instead use from/to dates
            "FromDate": from_date,
            "ToDate": to_date,
            "TimeType": req.time_type.to_string(),
            "DistanceId": req.distance,
            "StrokeId": req.stroke as u8,
            "CourseId": req.course as u8,
            "StartAge": start_age,
            "EndAge": end_age,
            "Gender": req.gender.to_string(),
            "Standard": "12",  // corresponds to "slower than B", taken from dropdown menu index (probably unstable)
            "IncludeTimesForUsaSwimmingMembersOnly": members_only,
            "ClubId": "-1",  // TODO
            "ClubName": "",  // TODO
            "Lscs": "All",  // TODO: "All" if lscs is None else "+".join(lscs)
            "Zone": req.zone as u8,
            "TimesToInclude": best_only,
            "SortBy1": "EventSortOrder",
            "SortBy2": "",
            "SortBy3": "",
            "MaxResults": req.max_results,
        });
        value
    }
}
