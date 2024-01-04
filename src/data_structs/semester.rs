use serde::{Deserialize, Serialize};
use sqlx::Decode;

#[derive(Debug, PartialEq, Eq)]
#[derive(Deserialize, Serialize, Decode)]
pub enum SemesterSeason {
    Summer1,
    Summer2,
    Fall,
    Spring
}

#[derive(Debug, PartialEq, Eq)]
#[derive(Deserialize, Serialize)]
pub struct Semester {
    pub semester_season: SemesterSeason,
    pub semester_year: u16,
}

impl SemesterSeason {
    pub fn to_string(&self) -> String {
        match self {
            SemesterSeason::Summer1 => "Summer1".to_string(),
            SemesterSeason::Summer2 => "Summer2".to_string(),
            SemesterSeason::Fall => "Fall".to_string(),
            SemesterSeason::Spring => "Spring".to_string(),
        }
    }

    pub fn from_string(season: &str) -> SemesterSeason {
        let lower_season = season.clone().to_lowercase();
        match lower_season.as_str() {
            "summer1" => SemesterSeason::Summer1,
            "summer2" => SemesterSeason::Summer2,
            "fall" => SemesterSeason::Fall,
            "spring" => SemesterSeason::Spring,
            _ => panic!("Invalid season string")
        }
    }
}