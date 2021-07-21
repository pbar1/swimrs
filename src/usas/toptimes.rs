// use chrono::offset::Local;
// use chrono::{Duration, NaiveDate};
// use serde_json::{json, Value};
//
// use super::model::*;
//
// #[derive(Debug)]
// pub struct TopTimesRequest {
//     gender: Gender,
//     distance: u16,
//     stroke: Stroke,
//     course: Course,
//     from_date: NaiveDate,
//     to_date: NaiveDate,
//     start_age: Option<u8>,
//     end_age: Option<u8>,
//     zone: Zone,
//     lscs: Vec<LSC>,
//     time_type: TimeType,
//     members_only: bool,
//     best_only: bool,
//     max_results: u16,
// }
//
// impl Default for TopTimesRequest {
//     fn default() -> TopTimesRequest {
//         TopTimesRequest {
//             gender: Gender::Mixed,
//             distance: 50,
//             stroke: Stroke::All,
//             course: Course::All,
//             from_date: Local::now().naive_local().date() - Duration::weeks(1),
//             to_date: Local::now().naive_local().date(),
//             start_age: None,
//             end_age: None,
//             zone: Zone::All,
//             lscs: vec![LSC::All],
//             time_type: TimeType::Individual,
//             members_only: false,
//             best_only: false,
//             max_results: 5000,
//         }
//     }
// }
//
// fn generate_request_body(req: TopTimesRequest) -> Value {
//     let start_age = match req.start_age {
//         Some(age) => age.to_string(),
//         None => "All".to_string(),
//     };
//     let end_age = match req.end_age {
//         Some(age) => age.to_string(),
//         None => "All".to_string(),
//     };
//     let members_only = match req.members_only {
//         true => "Yes",
//         false => "No",
//     };
//     let best_only = match req.best_only {
//         true => "Best",
//         false => "All",
//     };
//     json!({
//         "DivId": "Times_TimesSearchTopTimesEventRankSearch_Index_Div-1",  // constant value
//         "DateRangeId": "0",  // set to 0 to disable preset date range and instead use from/to dates
//         "FromDate": req.from_date.format("%-m/%-d/%Y").to_string(),
//         "ToDate": req.to_date.format("%-m/%-d/%Y").to_string(),
//         "TimeType": req.time_type.to_string(),
//         "DistanceId": req.distance,
//         "StrokeId": req.stroke as u8,
//         "CourseId": req.course as u8,
//         "StartAge": start_age,
//         "EndAge": end_age,
//         "Gender": req.gender.to_string(),
//         "Standard": "12",  // corresponds to "slower than B", taken from dropdown menu index (probably unstable)
//         "IncludeTimesForUsaSwimmingMembersOnly": members_only,
//         "ClubId": "-1",  // TODO
//         "ClubName": "",  // TODO
//         "Lscs": "All",  // TODO: "All" if lscs is None else "+".join(lscs)
//         "Zone": req.zone as u8,
//         "TimesToInclude": best_only,
//         "SortBy1": "EventSortOrder",
//         "SortBy2": "",
//         "SortBy3": "",
//         "MaxResults": request.max_results,
//     })
// }
//
// pub async fn top_times_raw(request: TopTimesRequest) -> Result<String, Box<dyn Error>> {
//     let report_key_req_body = generate_request_body(request);
//
//     // Submit the request to an endpoint that generates a key that refers to a CSV report
//     // TODO: Is this key deterministic?
//     let report_key = self
//         .http_client
//         .post(format!(
//             "{}{}",
//             BASE_URL, "/times/popular-resources/event-rank-search/CsvTimes"
//         ))
//         .json(&report_key_req_body)
//         .send()
//         .await?
//         .text()
//         .await?;
//
//     // Exchange the key for the real CSV report
//     let csv_raw = self
//         .http_client
//         .get(format!(
//             "{}{}",
//             BASE_URL, "/api/Reports_ReportViewer/GetReport"
//         ))
//         .query(&[
//             ("Key", report_key),
//             ("Format", "Csv".to_string()),
//             ("IsFileDownload", "false".to_string()),
//         ])
//         .send()
//         .await?
//         .text()
//         .await?;
//
//     match csv_raw.contains("Please rerun the report.") {
//         true => Err("unable to fetch top times".into()),
//         false => Ok(csv_raw),
//     }
// }
//
// mod test {}
