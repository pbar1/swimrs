use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
    str::FromStr,
};

use anyhow::{Context, Result};
use chrono::{offset::Local, NaiveDate};
use itertools::Itertools;
use maplit::hashmap;
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::common::{Course, Distance, Gender, Stroke, SwimEvent, SwimTime, TimeType, Zone, LSC};

const DATE_FMT: &str = "%-m/%-d/%Y";
const URL_PAGE: &str = "https://www.usaswimming.org/times/popular-resources/event-rank-search";
const URL_API: &str =
    "https://www.usaswimming.org/api/Times_TimesSearchTopTimesEventRankSearch/ListTimes";

#[derive(Debug, Clone)]
pub struct TopTimesClient {
    client: Client,
}

/// Input for Top Times / Event Rank Search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopTimesRequest {
    pub gender: Gender,
    pub distance: Distance,
    pub stroke: Stroke,
    pub course: Course,
    pub from_date: NaiveDate,
    pub to_date: NaiveDate,
    pub start_age: Option<u8>,
    pub end_age: Option<u8>,
    pub zone: Zone,
    pub lscs: Option<Vec<LSC>>,
    pub time_type: TimeType,
    pub members_only: bool,
    pub best_only: bool,
    pub max_results: u32,
}

#[derive(Debug, Serialize)]
pub struct TopTime {
    pub age: u8,
    pub course: Course,
    pub date: NaiveDate,
    pub distance: Distance,
    pub foreign: Option<bool>,
    pub gender: Gender,
    pub lsc: Option<LSC>,
    pub meet_id: Option<usize>,
    pub meet_name: String,
    pub power_points: Option<u16>,
    pub rank: Option<usize>,
    pub relay: bool,
    pub sanctioned: Option<bool>,
    pub stroke: Stroke,
    pub swimmer_id: Option<usize>,
    pub swimmer_name: String,
    pub team_name: String,
    pub time: f32,
    pub time_alt_adj: Option<f32>,
    pub time_id: Option<usize>,
    pub time_standard: Option<String>,
}

// 0----1----2---------3-------4---5---6-----7---------8---------9-------------10---------11
// rank|time|full_name|foreign|age|lsc|event|team_name|meet_name|time_standard|sanctioned|script
pub fn parse_top_times(raw_html: String, gender: Gender) -> Result<Vec<TopTime>> {
    let dom = tl::parse(&raw_html, tl::ParserOptions::default())?;
    let parser = dom.parser();
    dom.query_selector("td.usas-hide-mobile")
        .context("error parsing selector")?
        .filter_map(|x| x.get(parser))
        .tuples::<(_, _, _, _, _, _, _, _, _, _, _, _)>()
        .map(|r| {
            let rank = Some(r.0.inner_text(parser).parse::<usize>()?);
            let SwimTime { seconds, relay } = SwimTime::from_str(&r.1.inner_text(parser))?;
            let swimmer_name = r.2.inner_text(parser).trim().replace("<br>", "");
            let foreign = Some(r.3.inner_text(parser) == "Yes");
            let age = r.4.inner_text(parser).parse::<u8>()?;
            let lsc = Some(LSC::from_str(&r.5.inner_text(parser))?);
            let SwimEvent(distance, stroke, course) = SwimEvent::from_str(&r.6.inner_text(parser))?;
            let team_name = r.7.inner_text(parser).to_string();
            let meet_name = r.8.inner_text(parser).to_string();
            let time_standard = Some(r.9.inner_text(parser).to_string());
            let sanctioned = Some(r.10.inner_text(parser) == "Yes");

            // FIXME: Parse the script block for these
            // let script = row.get(11).context("no script")?;
            // let split = script.split(',').collect::<String>().trim;
            let swimmer_id = Some(0usize);
            let meet_id = Some(0usize);
            let date = NaiveDate::from_ymd(2020, 2, 20);

            let top_time = TopTime {
                age,
                course,
                date,
                distance,
                foreign,
                gender: gender.clone(),
                lsc,
                meet_id,
                meet_name,
                power_points: None,
                rank,
                relay,
                sanctioned,
                stroke,
                swimmer_id,
                swimmer_name,
                team_name,
                time: seconds,
                time_alt_adj: None,
                time_id: None,
                time_standard,
            };
            Ok(top_time)
        })
        .collect::<Result<Vec<TopTime>>>()
}

impl TopTimesClient {
    /// Creates a TopTimesClient from the provided Reqwest client builder.
    /// Enables the cookie jar, which is required for HTTP requests to
    /// succeed.
    pub fn new(builder: ClientBuilder) -> Result<Self> {
        let client = builder.cookie_store(true).build()?;
        Ok(TopTimesClient { client })
    }

    /// Visits the USA Swimming Top Times / Event Rank Search landing page. This
    /// populates the HTTP client's cookie jar with cookies necessary for
    /// Top Times searches to succeed.
    pub async fn populate_cookies(&self) -> Result<()> {
        self.client.get(URL_PAGE).send().await?.error_for_status()?;
        Ok(())
    }

    /// Performs a USA Swimming Top Times / Event Rank Search using the given
    /// request parameters and returns the raw HTML response.
    pub async fn fetch_html(&self, req: TopTimesRequest) -> Result<String> {
        let form = HashMap::from(req);
        let resp = self
            .client
            .post(URL_API)
            .form(&form)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        Ok(resp)
    }

    /// Performs a USA Swimming Top Times / Event Rank Search using the given
    /// request parameters and returns a list of parsed times.
    pub async fn fetch_top_times(&self, req: TopTimesRequest) -> Result<Vec<TopTime>> {
        let gender = req.gender.clone();
        let raw_html = self.fetch_html(req).await?;
        parse_top_times(raw_html, gender)
    }
}

impl Default for TopTimesRequest {
    /// Creates a default Top Times / Event Rank Search request. Date range is
    /// the current date only. Includes all distances, strokes, courses,
    /// ages, Zones, LSCs; both members and non-members; all times for each
    /// swimmer. Searches for individual times. Limits results to a total of
    /// 50000.
    fn default() -> TopTimesRequest {
        TopTimesRequest {
            gender: Gender::Mixed,
            distance: Distance::All,
            stroke: Stroke::All,
            course: Course::All,
            from_date: Local::now().naive_local().date(),
            to_date: Local::now().naive_local().date(),
            start_age: None,
            end_age: None,
            zone: Zone::All,
            lscs: None,
            time_type: TimeType::Individual,
            members_only: false,
            best_only: false,
            max_results: 50000,
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
        let lscs = match &self.lscs {
            Some(l) => l
                .into_iter()
                .map(|lsc| lsc.to_string())
                .collect::<Vec<String>>()
                .join("+"),
            None => "All".to_owned(),
        };

        write!(
            f,
            "{}/{}_{}_{}/{}_{}/{}_{}_{}_{}/{}_{}/{}_{}",
            self.gender,
            self.course,
            self.stroke,
            self.distance,
            self.zone,
            lscs,
            self.time_type,
            self.members_only,
            self.best_only,
            self.max_results,
            start_age,
            end_age,
            self.from_date,
            self.to_date,
        )
    }
}

impl From<TopTimesRequest> for HashMap<&str, String> {
    fn from(req: TopTimesRequest) -> Self {
        debug!("Converting TopTimesRequest to HashMap: {:?}", req);

        let start_age = match req.start_age {
            Some(age) => age.to_string(),
            None => "All".to_owned(),
        };
        let end_age = match req.end_age {
            Some(age) => age.to_string(),
            None => "All".to_owned(),
        };
        let members_only = match req.members_only {
            true => "Yes".to_owned(),
            false => "No".to_owned(),
        };
        let best_only = match req.best_only {
            true => "Best".to_owned(),
            false => "All".to_owned(),
        };
        let lscs = match req.lscs {
            Some(l) => l
                .into_iter()
                .map(|lsc| lsc.to_string())
                .collect::<Vec<String>>()
                .join("+"),
            None => "All".to_owned(),
        };
        let from_date = req.from_date.format(DATE_FMT).to_string();
        let to_date = req.to_date.format(DATE_FMT).to_string();

        hashmap! {
            "DivId" => "Times_TimesSearchTopTimesEventRankSearch_Index_Div-1".to_owned(),
            "DateRangeId" => "0".to_owned(), // Disables preset date range
            "FromDate" => from_date,
            "ToDate" => to_date,
            "TimeType" => req.time_type.to_string(),
            "DistanceId" => (req.distance as u16).to_string(),
            "StrokeId" => (req.stroke as u8).to_string(),
            "CourseId" => (req.course as u8).to_string(),
            "StartAge" => start_age,
            "EndAge" => end_age,
            "Gender" => req.gender.to_string(),
            "Standard" => "12".to_owned(), // "Slower than B"
            "IncludeTimesForUsaSwimmingMembersOnly" => members_only,
            "ClubId" => "-1".to_owned(),  // TODO
            "ClubName" => "".to_owned(),  // TODO
            "Lscs" => lscs,
            "Zone" => (req.zone as u8).to_string(),
            "TimesToInclude" => best_only,
            "SortBy1" => "EventSortOrder".to_owned(),
            "SortBy2" => "".to_owned(),
            "SortBy3" => "".to_owned(),
            "MaxResults" => req.max_results.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_times_request_to_hashmap() {
        let req = TopTimesRequest {
            gender: Gender::Male,
            distance: Distance::_200,
            stroke: Stroke::Freestyle,
            course: Course::LCM,
            from_date: NaiveDate::from_ymd(2008, 8, 9),
            to_date: NaiveDate::from_ymd(2008, 8, 17),
            start_age: Some(23),
            end_age: None,
            zone: Zone::All,
            lscs: Some(vec![LSC::US, LSC::Unattached]),
            time_type: TimeType::Individual,
            members_only: false,
            best_only: false,
            max_results: 50000,
        };
        let mut req2 = req.clone();
        let map = HashMap::from(req);

        assert_eq!(map.get("FromDate").unwrap(), "8/9/2008");
        assert_eq!(map.get("ToDate").unwrap(), "8/17/2008");
        assert_eq!(map.get("TimeType").unwrap(), "Individual");
        assert_eq!(map.get("DistanceId").unwrap(), "200");
        assert_eq!(map.get("StrokeId").unwrap(), "1");
        assert_eq!(map.get("CourseId").unwrap(), "3");
        assert_eq!(map.get("StartAge").unwrap(), "23");
        assert_eq!(map.get("EndAge").unwrap(), "All");
        assert_eq!(map.get("Gender").unwrap(), "Male");
        assert_eq!(
            map.get("IncludeTimesForUsaSwimmingMembersOnly").unwrap(),
            "No"
        );
        assert_eq!(map.get("Lscs").unwrap(), "US+UN");
        assert_eq!(map.get("Zone").unwrap(), "0");
        assert_eq!(map.get("TimesToInclude").unwrap(), "All");
        assert_eq!(map.get("MaxResults").unwrap(), "50000");

        req2.lscs = None;
        let map = HashMap::from(req2);

        assert_eq!(map.get("Lscs").unwrap(), "All");
    }

    #[tokio::test]
    async fn test_fetch_top_times() {
        let client = TopTimesClient::new(Client::builder()).unwrap();
        client.populate_cookies().await.unwrap();

        let req = TopTimesRequest {
            gender: Gender::Male,
            distance: Distance::_200,
            stroke: Stroke::Freestyle,
            course: Course::LCM,
            from_date: NaiveDate::from_ymd(2008, 8, 9),
            to_date: NaiveDate::from_ymd(2008, 8, 17),
            start_age: Some(23),
            end_age: Some(23),
            zone: Zone::All,
            lscs: None,
            time_type: TimeType::Individual,
            members_only: false,
            best_only: false,
            max_results: 50000,
        };
        let times = client.fetch_top_times(req).await.unwrap();

        assert_eq!(times.get(0).unwrap().swimmer_name, "Phelps, Michael");
    }

    #[test]
    fn test_parse_top_times_small() {
        let html = std::fs::read_to_string("testdata/top_times_small.html").unwrap();
        let times = parse_top_times(html, Gender::Male).unwrap();

        let first = times.first().unwrap();
        assert_eq!(first.swimmer_name, "Phelps, Michael");
        assert!((first.time - 102.96).abs() < 0.01);

        let last = times.last().unwrap();
        assert_eq!(last.swimmer_name, "Mebarek, Mahrez");
        assert!((last.time - 112.66).abs() < 0.01);
    }

    #[test]
    fn test_parse_top_times_large() {
        let html = std::fs::read_to_string("testdata/top_times_large.html").unwrap();
        let times = parse_top_times(html, Gender::Female).unwrap();

        let first = times.first().unwrap();
        assert_eq!(first.swimmer_name, "Zielinski, Logananne");
        assert!((first.time - 26.58).abs() < 0.01);

        let last = times.last().unwrap();
        assert_eq!(last.swimmer_name, "Olson, Kennedy");
        assert!((last.time - 431.72).abs() < 0.01);
    }
}
