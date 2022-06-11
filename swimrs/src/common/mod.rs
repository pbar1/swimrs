use std::str::FromStr;

use anyhow::{bail, Error, Result};
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use tracing::debug;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Display)]
#[serde(rename_all = "PascalCase")]
pub enum Gender {
    Male,
    Female,
    Mixed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Display, TryFromPrimitive)]
#[repr(u16)]
pub enum Distance {
    All = 0,
    _50 = 50,
    _100 = 100,
    _200 = 200,
    _400 = 400,
    _500 = 500,
    _800 = 800,
    _1000 = 1000,
    _1500 = 1500,
    _1650 = 1650,
}

// TODO: Why does this have both rename and serialize?
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Display, EnumString)]
pub enum Stroke {
    All = 0,

    #[serde(rename = "FR")]
    #[strum(serialize = "FR")]
    Freestyle = 1,

    #[serde(rename = "BK")]
    #[strum(serialize = "BK")]
    Backstroke = 2,

    #[serde(rename = "BR")]
    #[strum(serialize = "BR")]
    Breaststroke = 3,

    #[serde(rename = "FL")]
    #[strum(serialize = "FL")]
    Butterfly = 4,

    #[serde(rename = "IM")]
    #[strum(serialize = "IM")]
    IndividualMedley = 5,

    #[serde(rename = "FR-R")]
    #[strum(serialize = "FR-R")]
    FreestyleRelay = 6,

    #[serde(rename = "MED-R")]
    #[strum(serialize = "MED-R")]
    MedleyRelay = 7,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Display, EnumString)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Display)]
pub enum Zone {
    All = 0,
    Central = 1,
    Eastern = 2,
    Southern = 3,
    Western = 4,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Display, EnumString)]
pub enum LSC {
    #[strum(serialize = "All")]
    All,
    #[strum(serialize = "UN")]
    Unattached,
    #[strum(serialize = "AD")]
    AD,
    #[strum(serialize = "AK")]
    AK,
    #[strum(serialize = "AM")]
    AM,
    #[strum(serialize = "AZ")]
    AZ,
    #[strum(serialize = "AR")]
    AR,
    #[strum(serialize = "BD")]
    BD,
    #[strum(serialize = "CC")]
    CC,
    #[strum(serialize = "CO")]
    CO,
    #[strum(serialize = "CT")]
    CT,
    #[strum(serialize = "FG")]
    FG,
    #[strum(serialize = "FL")]
    FL,
    #[strum(serialize = "GA")]
    GA,
    #[strum(serialize = "GU")]
    GU,
    #[strum(serialize = "HI")]
    HI,
    #[strum(serialize = "IL")]
    IL,
    #[strum(serialize = "IN")]
    IN,
    #[strum(serialize = "IE")]
    IE,
    #[strum(serialize = "IA")]
    IA,
    #[strum(serialize = "KY")]
    KY,
    #[strum(serialize = "LE")]
    LE,
    #[strum(serialize = "LA")]
    LA,
    #[strum(serialize = "ME")]
    ME,
    #[strum(serialize = "MD")]
    MD,
    #[strum(serialize = "MR")]
    MR,
    #[strum(serialize = "MI")]
    MI,
    #[strum(serialize = "MA")]
    MA,
    #[strum(serialize = "MW")]
    MW,
    #[strum(serialize = "MN")]
    MN,
    #[strum(serialize = "MS")]
    MS,
    #[strum(serialize = "MV")]
    MV,
    #[strum(serialize = "MT")]
    MT,
    #[strum(serialize = "NE")]
    NE,
    #[strum(serialize = "NJ")]
    NJ,
    #[strum(serialize = "NM")]
    NM,
    #[strum(serialize = "NI")]
    NI,
    #[strum(serialize = "NC")]
    NC,
    #[strum(serialize = "ND")]
    ND,
    #[strum(serialize = "NT")]
    NT,
    #[strum(serialize = "OH")]
    OH,
    #[strum(serialize = "OK")]
    OK,
    #[strum(serialize = "OR")]
    OR,
    #[strum(serialize = "OZ")]
    OZ,
    #[strum(serialize = "PN")]
    PN,
    #[strum(serialize = "PC")]
    PC,
    #[strum(serialize = "PV")]
    PV,
    #[strum(serialize = "SI")]
    SI,
    #[strum(serialize = "SN")]
    SN,
    #[strum(serialize = "SR")]
    SR,
    #[strum(serialize = "SC")]
    SC,
    #[strum(serialize = "SD")]
    SD,
    #[strum(serialize = "ST")]
    ST,
    #[strum(serialize = "SE")]
    SE,
    #[strum(serialize = "CA")]
    CA,
    #[strum(serialize = "US")]
    US,
    #[strum(serialize = "UT")]
    UT,
    #[strum(serialize = "VA")]
    VA,
    #[strum(serialize = "WT")]
    WT,
    #[strum(serialize = "WV")]
    WV,
    #[strum(serialize = "WI")]
    WI,
    #[strum(serialize = "WY")]
    WY,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Display)]
pub enum TimeType {
    Individual,
    Relay,
}

#[derive(Debug, PartialEq)]
pub struct SwimEvent(pub Distance, pub Stroke, pub Course);

#[derive(Debug, PartialEq)]
pub struct SwimTime {
    pub seconds: f32,
    pub relay: bool,
}

impl FromStr for SwimEvent {
    type Err = Error;

    /// Converts a string like "100 FR SCY" to a SwimEvent.
    fn from_str(s: &str) -> Result<Self> {
        debug!("Converting to SwimEvent: {}", s);

        let split: Vec<&str> = s.split(' ').collect();
        if split.len() != 3 {
            bail!("Unexpected SwimEvent str: {}", s);
        }
        let distance = Distance::try_from_primitive(split[0].parse::<u16>()?)?;
        let stroke: Stroke = Stroke::from_str(split[1])?;
        let course: Course = Course::from_str(split[2])?;

        Ok(SwimEvent(distance, stroke, course))
    }
}

impl FromStr for SwimTime {
    type Err = Error;

    /// Converts a string like "19.79", "19.79r", "1:04.02", "1:04.02r" to a SwimTime.
    fn from_str(s: &str) -> Result<Self> {
        debug!("Converting to SwimTime: {}", s);

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
            _ => bail!("Unexpected SwimTime str: {}", s),
        };

        Ok(SwimTime { seconds, relay })
    }
}

pub const VALID_EVENTS: [SwimEvent; 53] = [
    // SCY
    SwimEvent(Distance::_50, Stroke::Freestyle, Course::SCY),
    SwimEvent(Distance::_100, Stroke::Freestyle, Course::SCY),
    SwimEvent(Distance::_200, Stroke::Freestyle, Course::SCY),
    SwimEvent(Distance::_500, Stroke::Freestyle, Course::SCY),
    SwimEvent(Distance::_1000, Stroke::Freestyle, Course::SCY),
    SwimEvent(Distance::_1650, Stroke::Freestyle, Course::SCY),
    SwimEvent(Distance::_50, Stroke::Backstroke, Course::SCY),
    SwimEvent(Distance::_100, Stroke::Backstroke, Course::SCY),
    SwimEvent(Distance::_200, Stroke::Backstroke, Course::SCY),
    SwimEvent(Distance::_50, Stroke::Breaststroke, Course::SCY),
    SwimEvent(Distance::_100, Stroke::Breaststroke, Course::SCY),
    SwimEvent(Distance::_200, Stroke::Breaststroke, Course::SCY),
    SwimEvent(Distance::_50, Stroke::Butterfly, Course::SCY),
    SwimEvent(Distance::_100, Stroke::Butterfly, Course::SCY),
    SwimEvent(Distance::_200, Stroke::Butterfly, Course::SCY),
    SwimEvent(Distance::_100, Stroke::IndividualMedley, Course::SCY),
    SwimEvent(Distance::_200, Stroke::IndividualMedley, Course::SCY),
    SwimEvent(Distance::_400, Stroke::IndividualMedley, Course::SCY),
    // SCM
    SwimEvent(Distance::_50, Stroke::Freestyle, Course::SCM),
    SwimEvent(Distance::_100, Stroke::Freestyle, Course::SCM),
    SwimEvent(Distance::_200, Stroke::Freestyle, Course::SCM),
    SwimEvent(Distance::_400, Stroke::Freestyle, Course::SCM),
    SwimEvent(Distance::_800, Stroke::Freestyle, Course::SCM),
    SwimEvent(Distance::_1500, Stroke::Freestyle, Course::SCM),
    SwimEvent(Distance::_50, Stroke::Backstroke, Course::SCM),
    SwimEvent(Distance::_100, Stroke::Backstroke, Course::SCM),
    SwimEvent(Distance::_200, Stroke::Backstroke, Course::SCM),
    SwimEvent(Distance::_50, Stroke::Breaststroke, Course::SCM),
    SwimEvent(Distance::_100, Stroke::Breaststroke, Course::SCM),
    SwimEvent(Distance::_200, Stroke::Breaststroke, Course::SCM),
    SwimEvent(Distance::_50, Stroke::Butterfly, Course::SCM),
    SwimEvent(Distance::_100, Stroke::Butterfly, Course::SCM),
    SwimEvent(Distance::_200, Stroke::Butterfly, Course::SCM),
    SwimEvent(Distance::_100, Stroke::IndividualMedley, Course::SCM),
    SwimEvent(Distance::_200, Stroke::IndividualMedley, Course::SCM),
    SwimEvent(Distance::_400, Stroke::IndividualMedley, Course::SCM),
    // LCM
    SwimEvent(Distance::_50, Stroke::Freestyle, Course::LCM),
    SwimEvent(Distance::_100, Stroke::Freestyle, Course::LCM),
    SwimEvent(Distance::_200, Stroke::Freestyle, Course::LCM),
    SwimEvent(Distance::_400, Stroke::Freestyle, Course::LCM),
    SwimEvent(Distance::_800, Stroke::Freestyle, Course::LCM),
    SwimEvent(Distance::_1500, Stroke::Freestyle, Course::LCM),
    SwimEvent(Distance::_50, Stroke::Backstroke, Course::LCM),
    SwimEvent(Distance::_100, Stroke::Backstroke, Course::LCM),
    SwimEvent(Distance::_200, Stroke::Backstroke, Course::LCM),
    SwimEvent(Distance::_50, Stroke::Breaststroke, Course::LCM),
    SwimEvent(Distance::_100, Stroke::Breaststroke, Course::LCM),
    SwimEvent(Distance::_200, Stroke::Breaststroke, Course::LCM),
    SwimEvent(Distance::_50, Stroke::Butterfly, Course::LCM),
    SwimEvent(Distance::_100, Stroke::Butterfly, Course::LCM),
    SwimEvent(Distance::_200, Stroke::Butterfly, Course::LCM),
    SwimEvent(Distance::_200, Stroke::IndividualMedley, Course::LCM),
    SwimEvent(Distance::_400, Stroke::IndividualMedley, Course::LCM),
];
