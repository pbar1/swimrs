#[derive(strum::Display)]
pub enum Gender {
    Male,
    Female,
    Mixed,
}

#[derive(strum::Display)]
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

#[derive(strum::Display)]
pub enum Course {
    All = 0,
    LCM = 1,
    SCM = 2,
    SCY = 3,
}

#[derive(strum::Display)]
pub enum Zone {
    All = 0,
    Central = 1,
    Eastern = 2,
    Southern = 3,
    Western = 4,
}

// TODO: implement rest of the LSCs
#[derive(strum::Display)]
pub enum LSC {
    All,
}

#[derive(strum::Display)]
pub enum TimeType {
    Individual,
    Relay,
}
