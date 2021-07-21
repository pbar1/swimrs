use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(Debug, Serialize, Deserialize, Display, EnumString)]
pub enum Gender {
    #[serde(rename = "Male")]
    Male,

    #[serde(rename = "Female")]
    Female,

    #[serde(rename = "Mixed")]
    Mixed,
}

#[derive(Debug, Serialize, Deserialize, Display, EnumString)]
pub enum Stroke {
    All = 0,
    FR = 1,
    BK = 2,
    BR = 3,
    FL = 4,
    IM = 5,
    FR_R = 6,
    MED_R = 7,
}

#[derive(Debug, Serialize, Deserialize, Display, EnumString)]
pub enum Course {
    All = 0,
    LCM = 1,
    SCM = 2,
    SCY = 3,
}

#[derive(Debug, Serialize, Deserialize, Display, EnumString)]
pub enum Zone {
    All = 0,
    Central = 1,
    Eastern = 2,
    Southern = 3,
    Western = 4,
}

// TODO: implement rest of the LSCs
#[derive(Debug, Serialize, Deserialize, Display, EnumString)]
pub enum LSC {
    All,
}

#[derive(Debug, Serialize, Deserialize, Display, EnumString)]
pub enum TimeType {
    Individual,
    Relay,
}
