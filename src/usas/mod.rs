use chrono::{Duration, NaiveDate};
use serde_json::json;

use model::{Course, Gender, Stroke, TimeType, Zone, LSC};

pub mod model;

const BASE_URL: &str = "https://www.usaswimming.org";

pub struct TopTimesRequest {
    pub(crate) gender: Gender,
    pub(crate) distance: u16,
    pub(crate) stroke: Stroke,
    pub(crate) course: Course,
    pub(crate) from_date: NaiveDate,
    pub(crate) to_date: NaiveDate,
    pub(crate) start_age: Option<u8>,
    pub(crate) end_age: Option<u8>,
    pub(crate) zone: Zone,
    pub(crate) lscs: Vec<LSC>,
    pub(crate) time_type: TimeType,
    pub(crate) members_only: bool,
    pub(crate) best_only: bool,
    pub(crate) max_results: u16,
}

impl Default for TopTimesRequest {
    fn default() -> TopTimesRequest {
        TopTimesRequest {
            gender: Gender::Mixed,
            distance: 50,
            stroke: Stroke::All,
            course: Course::All,
            from_date: chrono::offset::Local::now().naive_local().date() - Duration::weeks(1),
            to_date: chrono::offset::Local::now().naive_local().date(),
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

pub struct USASClient {
    http_client: reqwest::Client,
}

impl USASClient {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let http_client = reqwest::Client::builder().cookie_store(true).build()?;
        http_client
            .get(format!(
                "{}{}",
                BASE_URL, "/times/popular-resources/event-rank-search"
            ))
            .send()
            .await?;
        Ok(USASClient { http_client })
    }

    pub async fn top_times_raw(
        &self,
        request: TopTimesRequest,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let start_age = match request.start_age {
            Some(age) => age.to_string(),
            None => "All".to_string(),
        };
        let end_age = match request.end_age {
            Some(age) => age.to_string(),
            None => "All".to_string(),
        };
        let members_only = match request.members_only {
            true => "Yes",
            false => "No",
        };
        let best_only = match request.best_only {
            true => "Best",
            false => "All",
        };

        let report_key_req_body = json!({
            "DivId": "Times_TimesSearchTopTimesEventRankSearch_Index_Div-1",  // constant value
            "DateRangeId": "0",  // set to 0 to disable preset date range and instead use from/to dates
            "FromDate": request.from_date.format("%-m/%-d/%Y").to_string(),
            "ToDate": request.to_date.format("%-m/%-d/%Y").to_string(),
            "TimeType": request.time_type.to_string(),
            "DistanceId": request.distance,
            "StrokeId": request.stroke as u8,
            "CourseId": request.course as u8,
            "StartAge": start_age,
            "EndAge": end_age,
            "Gender": request.gender.to_string(),
            "Standard": "12",  // corresponds to "slower than B", taken from dropdown menu index (probably unstable)
            "IncludeTimesForUsaSwimmingMembersOnly": members_only,
            "ClubId": "-1",  // TODO
            "ClubName": "",  // TODO
            "Lscs": "All",  // TODO: "All" if lscs is None else "+".join(lscs)
            "Zone": request.zone as u8,
            "TimesToInclude": best_only,
            "SortBy1": "EventSortOrder",
            "SortBy2": "",
            "SortBy3": "",
            "MaxResults": request.max_results,
        });

        // Submit the request to an endpoint that generates a key that refers to a CSV report
        // TODO: Is this key deterministic?
        let report_key = self
            .http_client
            .post(format!(
                "{}{}",
                BASE_URL, "/times/popular-resources/event-rank-search/CsvTimes"
            ))
            .json(&report_key_req_body)
            .send()
            .await?
            .text()
            .await?;

        // Exchange the key for the real CSV report
        let csv_raw = self
            .http_client
            .get(format!(
                "{}{}",
                BASE_URL, "/api/Reports_ReportViewer/GetReport"
            ))
            .query(&[
                ("Key", report_key),
                ("Format", "Csv".to_string()),
                ("IsFileDownload", "false".to_string()),
            ])
            .send()
            .await?
            .text()
            .await?;

        match csv_raw.contains("Please rerun the report.") {
            true => Err("unable to fetch top times".into()),
            false => Ok(csv_raw),
        }
    }
}

pub async fn test_fn() -> Result<(), Box<dyn std::error::Error>> {
    let usas_client = USASClient::new().await?;

    let top_times_req = TopTimesRequest {
        gender: Gender::Male,
        distance: 50,
        stroke: Stroke::FR,
        course: Course::LCM,
        from_date: chrono::NaiveDate::from_ymd(2021, 01, 01),
        to_date: chrono::offset::Local::now().naive_local().date(),
        start_age: Some(20),
        end_age: Some(25),
        zone: Zone::All,
        lscs: vec![LSC::All],
        time_type: TimeType::Individual,
        members_only: false,
        best_only: false,
        max_results: 5000,
    };

    let output = usas_client.top_times_raw(top_times_req).await?;

    println!("{}", output);

    Ok(())
}
