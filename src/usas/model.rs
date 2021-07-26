use std::{error::Error, str::FromStr};

use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Display)]
#[serde(rename_all = "PascalCase")]
pub enum Gender {
    Male,
    Female,
    Mixed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Display, EnumString)]
pub enum Stroke {
    /// All strokes
    All = 0,

    /// Freestyle
    #[serde(rename = "FR")]
    #[strum(serialize = "FR")]
    Freestyle = 1,

    /// Backstroke
    #[serde(rename = "BK")]
    #[strum(serialize = "BK")]
    Backstroke = 2,

    /// Breaststroke
    #[serde(rename = "BR")]
    #[strum(serialize = "BR")]
    Breaststroke = 3,

    /// Butterfly
    #[serde(rename = "FL")]
    #[strum(serialize = "FL")]
    Butterfly = 4,

    /// Individual medley
    #[serde(rename = "IM")]
    #[strum(serialize = "IM")]
    IndividualMedley = 5,

    /// Freestyle relay
    #[serde(rename = "FR-R")]
    #[strum(serialize = "FR-R")]
    FreestyleRelay = 6,

    /// Medley relay
    #[serde(rename = "MED-R")]
    #[strum(serialize = "MED-R")]
    MedleyRelay = 7,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Display, EnumString)]
pub enum Course {
    /// All courses
    All = 0,

    /// Short course yards
    #[strum(serialize = "SCY")]
    SCY = 1,

    /// Short course meters
    #[strum(serialize = "SCM")]
    SCM = 2,

    /// Long course meters
    #[strum(serialize = "LCM")]
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

pub struct SwimEvent {
    pub distance: u16,
    pub stroke: Stroke,
    pub course: Course,
}

pub struct SwimTime {
    pub seconds: f32,
    pub relay: bool,
}

impl FromStr for SwimEvent {
    type Err = Box<dyn Error>;

    /// Converts a string like "100 FR SCY" to a SwimEvent.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let split: Vec<&str> = s.split(' ').collect();
        if split.len() != 3 {
            return Err(format!("Unexpected SwimEvent str: {}", s).into());
        }

        let distance = split[0].parse::<u16>()?;
        let stroke: Stroke = Stroke::from_str(split[1])?;
        let course: Course = Course::from_str(split[2])?;

        Ok(SwimEvent {
            distance,
            stroke,
            course,
        })
    }
}

impl FromStr for SwimTime {
    type Err = Box<dyn Error>;

    /// Converts a string like "19.79", "19.79r", "1:04.02", "1:04.02r" to a SwimTime.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let relay = s.contains('r');
        let clean = s.replace('r', "");
        let split: Vec<&str> = clean.split(':').collect();

        let seconds = match split.len() {
            1 => split[0].parse::<f32>()?,
            2 => {
                let minutes = split[0].parse::<f32>()?;
                let seconds = split[1].parse::<f32>()?;
                60.0 * minutes + seconds
            }
            _ => return Err(format!("Unexpected SwimTime str: {}", s).into()),
        };

        Ok(SwimTime { seconds, relay })
    }
}
