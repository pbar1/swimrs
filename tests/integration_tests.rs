use chrono::NaiveDate;
use swimrs::usas::{
    indtimes,
    model::{Course, Gender, Stroke, SwimError, TimeType, Zone, LSC},
    toptimes,
};

// #[tokio::test]
// async fn individual_times() {
//     let req = indtimes::IndTimesRequest {
//         first_name: String::from("Caeleb"),
//         last_name: String::from("Dressel"),
//         from_date: NaiveDate::from_ymd(2019, 7, 26),
//         to_date: NaiveDate::from_ymd(2019, 7, 26),
//         distance: 100,
//         stroke: Stroke::Butterfly,
//         course: Course::LCM,
//         ..indtimes::IndTimesRequest::default()
//     };
//     let output = indtimes::get_times(req).await.unwrap();
//     let seconds = output[0].swim_time;
//     assert!((seconds - 49.50).abs() < 0.01);
// }

#[tokio::test]
async fn top_times() {
    let req = toptimes::TopTimesRequest {
        gender: Gender::Male,
        distance: 200,
        stroke: Stroke::Freestyle,
        course: Course::LCM,
        from_date: NaiveDate::from_ymd(2008, 1, 1),
        to_date: NaiveDate::from_ymd(2008, 12, 30),
        start_age: Some(22),
        end_age: Some(28),
        zone: Zone::All,
        lscs: vec![LSC::All],
        time_type: TimeType::Individual,
        members_only: false,
        best_only: false,
        max_results: 100,
    };
    let output = toptimes::search(req).await.unwrap();
    let seconds = output[0].swim_time_seconds;
    assert!((seconds - 102.96).abs() < 0.01);
}
