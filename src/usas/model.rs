use serde::{Deserialize, Serialize};
use strum::Display;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Display)]
#[serde(rename_all = "PascalCase")]
pub enum Gender {
    Male,
    Female,
    Mixed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Display)]
pub enum Stroke {
    /// All strokes
    All = 0,

    /// Freestyle
    #[serde(rename = "FR")]
    Freestyle = 1,

    /// Backstroke
    #[serde(rename = "BK")]
    Backstroke = 2,

    /// Breaststroke
    #[serde(rename = "BR")]
    Breaststroke = 3,

    /// Butterfly
    #[serde(rename = "FL")]
    Butterfly = 4,

    /// Individual medley
    #[serde(rename = "IM")]
    IndividualMedley = 5,

    /// Freestyle relay
    #[serde(rename = "FR-R")]
    FreestyleRelay = 6,

    /// Medley relay
    #[serde(rename = "MED-R")]
    MedleyRelay = 7,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Display)]
pub enum Course {
    /// All courses
    All = 0,

    /// Short course yards
    SCY = 1,

    /// Short course meters
    SCM = 2,

    /// Long course meters
    LCM = 3,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Display)]
pub enum Zone {
    All = 0,
    Central = 1,
    Eastern = 2,
    Southern = 3,
    Western = 4,
}

// TODO: implement rest of the LSCs
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Display)]
pub enum LSC {
    All,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Display)]
pub enum TimeType {
    Individual,
    Relay,
}

/// Errors that can be encountered.
#[derive(Debug, Error)]
pub enum SwimError {
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
