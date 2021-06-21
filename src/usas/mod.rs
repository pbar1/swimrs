use std::collections::HashMap;
use std::error::Error;

use chrono::offset::Local;
use chrono::{Duration, NaiveDate};
use regex::Regex;
use serde_json::json;

use self::model::{Course, Gender, Stroke, TimeType, Zone, LSC};

pub mod model;

const BASE_URL: &str = "https://www.usaswimming.org";

pub struct IndividualTimesRequest<'a> {
    first_name: &'a str,
    last_name: &'a str,
    from_date: NaiveDate,
    to_date: NaiveDate,
    distance: u16,
    stroke: Stroke,
    course: Course,
    start_age: Option<u8>,
    end_age: Option<u8>,
}

impl Default for IndividualTimesRequest<'_> {
    fn default() -> IndividualTimesRequest<'static> {
        IndividualTimesRequest {
            first_name: "*",
            last_name: "*",
            from_date: Local::now().naive_local().date() - Duration::days(365),
            to_date: Local::now().naive_local().date(),
            distance: 0,
            stroke: Stroke::All,
            course: Course::All,
            start_age: None,
            end_age: None,
        }
    }
}

pub struct TopTimesRequest {
    gender: Gender,
    distance: u16,
    stroke: Stroke,
    course: Course,
    from_date: NaiveDate,
    to_date: NaiveDate,
    start_age: Option<u8>,
    end_age: Option<u8>,
    zone: Zone,
    lscs: Vec<LSC>,
    time_type: TimeType,
    members_only: bool,
    best_only: bool,
    max_results: u16,
}

impl Default for TopTimesRequest {
    fn default() -> TopTimesRequest {
        TopTimesRequest {
            gender: Gender::Mixed,
            distance: 50,
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

pub struct USASClient {
    http_client: reqwest::Client,
}

impl USASClient {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
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

    pub async fn individual_times_raw(
        &self,
        req: IndividualTimesRequest<'_>,
    ) -> Result<String, Box<dyn Error>> {
        let start_age = match req.start_age {
            Some(age) => age.to_string(),
            None => "All".to_string(),
        };
        let end_age = match req.end_age {
            Some(age) => age.to_string(),
            None => "All".to_string(),
        };
        let from_date = req.from_date.format("%-m/%-d/%Y").to_string();
        let to_date = req.to_date.format("%-m/%-d/%Y").to_string();
        let distance_id = req.distance.to_string();
        let stroke_id = (req.stroke as u8).to_string();
        let course_id = (req.course as u8).to_string();

        let mut params = HashMap::new();
        params.insert("DivId", "Times_TimesSearchDetail_Index_Div-1");
        params.insert("FirstName", req.first_name);
        params.insert("LastName", req.last_name);
        params.insert("PersonId", "");
        params.insert("FromDate", from_date.as_str());
        params.insert("ToDate", to_date.as_str());
        params.insert("DateRangeId", "0");
        params.insert("DistanceId", distance_id.as_str());
        params.insert("StrokeId", stroke_id.as_str());
        params.insert("CourseId", course_id.as_str());
        params.insert("StartAge", start_age.as_str());
        params.insert("EndAge", end_age.as_str());
        params.insert("SelectedAgeFilter", "All");
        params.insert("SortPeopleBy", "Name");
        params.insert("SortTimesBy", "EventSortOrder");

        let req_url = format!("{}{}", BASE_URL, "/api/Times_TimesSearchDetail/ListTimes");
        let resp = self
            .http_client
            .post(req_url)
            .form(&params)
            .header("Cookie", r"BIGipServerPRODSFWEB_8085=!YZ5/13qbW3guCVguG6oy9/Z1oPNqRCW7wZjp6dwImlK28cIHX0po2nl/J37JkYWL4Kp6E0q0bew+jlQ=; sf-trckngckie=b0e400a3-f19c-4781-936f-0c548a1830d7; ASP.NET_SessionId=0ipn3yjw3q5l2y00izn5clpj; AKA_A2=A") // TODO: fetch this dynamically
            .send()
            .await?
            .text()
            .await?;

        let re = Regex::new(r"data: (\[.*])")?;
        let caps = re.captures(resp.as_str()).unwrap();
        let output = caps.get(1).map_or("", |m| m.as_str());

        Ok(output.to_string())
    }

    pub async fn top_times_raw(&self, request: TopTimesRequest) -> Result<String, Box<dyn Error>> {
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

pub async fn example_individual_times() -> Result<(), Box<dyn Error>> {
    let usas_client = USASClient::new().await?;

    let req = IndividualTimesRequest {
        first_name: "Ryan",
        last_name: "Lochte",
        from_date: NaiveDate::from_ymd(2016, 1, 1),
        to_date: NaiveDate::from_ymd(2016, 12, 30),
        ..IndividualTimesRequest::default()
    };

    let output = usas_client.individual_times_raw(req).await?;

    println!("{}", output);

    Ok(())
}

pub async fn example_top_times() -> Result<(), Box<dyn Error>> {
    let usas_client = USASClient::new().await?;

    let top_times_req = TopTimesRequest {
        gender: Gender::Male,
        distance: 50,
        stroke: Stroke::FR,
        course: Course::LCM,
        from_date: NaiveDate::from_ymd(2021, 01, 01),
        to_date: Local::now().naive_local().date(),
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
