use std::fmt::{Debug, Display};
use std::ops::{Add, Sub};
use std::str::FromStr;
use std::string::ParseError;
use chrono::{Datelike, Duration, NaiveDate, TimeZone};
use serde::{Deserialize, Serialize};
use sqlx::{Decode, Encode, MySql, Row, Type};
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::mysql::{MySqlTypeInfo, MySqlValueRef};

#[derive(Debug, PartialEq, Eq)]
#[derive(Deserialize, Serialize, sqlx::Type)]
#[derive(Clone)]
pub enum SemesterSeason {
    Summer1,
    Summer2,
    Fall,
    Spring
}

#[derive(Debug, PartialEq, Eq)]
#[derive(Deserialize, Serialize)]
#[derive(Clone)]
pub struct Semester {
    pub semester_season: SemesterSeason,
    pub semester_year: u16,
}

impl Semester {

    /// Get the ongoing semester and all semesters which will start with in the next 8 months
    pub fn get_current_and_upcoming_semesters() -> Vec<Semester> { //todo: update with approx registration instead
        // uses the current date to determine ongoing and upcoming semesters whose start is with in the next 8 months
        let current_date = chrono_tz::America::New_York.from_local_datetime(&chrono::Local::now().naive_local()).unwrap();
        let future_date = current_date.add(chrono::Duration::days(7 * 30));
        let current_or_upcoming_semester = Semester::get_current_or_upcoming_semester();
        let mut semesters: Vec<Semester> = Vec::new();
        semesters.push(current_or_upcoming_semester);

        let mut next_sem: Semester = semesters[semesters.len() - 1].get_next_semester();
        while next_sem.get_approx_start_date() < future_date.date_naive() {
            let curr_sem = next_sem.clone();
            semesters.push(curr_sem);
            next_sem = next_sem.get_next_semester();
        }

        semesters
    }

    pub fn get_current_or_upcoming_semester() -> Semester {
        let current_date = chrono_tz::America::New_York.from_local_datetime(&chrono::Local::now().naive_local()).unwrap();
        let current_season = SemesterSeason::get_current_season();
        if current_season.is_some() {
            return current_season.unwrap().to_semester(current_date.year() as u16);
        } else {
            let seasons = [SemesterSeason::Summer1, SemesterSeason::Summer2, SemesterSeason::Fall, SemesterSeason::Spring];
            let mut min_diff: (Option<&SemesterSeason>, Option<u16>) = (None, None);
            for season in seasons.iter() {
                let (start, _) = season.get_season_start_end();
                let date = NaiveDate::from_ymd(current_date.year(), start.month, start.day);
                let date_diff: Duration = date.sub(current_date.date_naive());
                if date_diff.num_days() >= 0 && (min_diff.1.is_none() || min_diff.1.unwrap() > date_diff.num_days() as u16) {
                    min_diff = (Option::from(season), Option::from(date_diff.num_days() as u16));
                }
            }
            if min_diff.0.is_none() {
                return SemesterSeason::Spring.to_semester((current_date.year() + 1) as u16);
            } else {
                let season = min_diff.0.unwrap().to_owned();
                return season.to_semester(current_date.year() as u16)
            }
        }
    }

    pub fn get_approx_start_date(&self) -> NaiveDate {
        let (start_date, _ ) = self.semester_season.get_season_start_end();
        return NaiveDate::from_ymd(self.semester_year as i32, start_date.month, start_date.day);
    }

    pub fn get_approx_end_date(&self) -> NaiveDate {
        let (_, end_date ) = self.semester_season.get_season_start_end();
        return NaiveDate::from_ymd(self.semester_year as i32, end_date.month, end_date.day);
    }

    /*pub async fn get_current_semesters() -> Vec<Semester> {
        // queries and parses the html for https://www.bu.edu/reg/calendars/semester/ to get the current semesters
        // using scraper as the html parser
        let page_html_raw = reqwest::get("https://www.bu.edu/reg/calendars/semester/")
            .await.unwrap().text().await.unwrap();
        let page_html = scraper::Html::parse_document(page_html_raw.as_str());
        let semester_selector = Selector::parse("div.bu_collapsible_container").unwrap();
        let results = page_html.select(&semester_selector);
        let mut current_semesters: Vec<Semester> = Vec::new();
        for result in results {
            let semester_name_selector = Selector::parse("h3.bu_collapsible").unwrap();
            let semester_name = result.select(&semester_name_selector).next().unwrap().text().next().unwrap();
            let event_row_selector = Selector::parse("tr").unwrap();
            let event_rows = result.select(&event_row_selector);
            // if the data listed in the last row is either today or in the future, then it is the current semester
            let mut is_not_past_semester = false;
            let current_semester = Semester::from_string(semester_name);
            for event_row in event_rows {
                let event_date_selector = Selector::parse("th").unwrap();
                let mut event_date = {
                    let html_txt = event_row.select(&event_date_selector).next().unwrap().text();
                    let joined_str = html_txt.collect::<Vec<_>>().join("");
                    joined_str.as_str().trim().to_string()
                };
                let formatted_event_date = {
                    // possible formats:
                    // MM DD - MM DD, YYYY
                    // MM DD - MM DD YYYY
                    // MM DD - DD, YYYY
                    // MM DD - DD YYYY
                    // MM DD - MM DD
                    // MM DD - DD
                    // MM DD, YYYY
                    // MM DD YYYY
                    // MM DD
                    let event_date_vec = event_date.replace(",", "");
                    let event_date_vec = event_date_vec.split(" ").collect::<Vec<&str>>();
                    if event_date_vec.len() < 2 {
                        println!("Error parsing date: {:#?}", &event_date); // BU is inconsistent so can happen
                        continue;
                    }
                    event_date_vec[0].to_owned() + " " + event_date_vec[1] + " " + current_semester.semester_year.to_string().as_str()
                };
                // dates are in this format: MM DD YYYY
                let event_date = match chrono::NaiveDate::parse_from_str(&formatted_event_date, "%B %d %Y") {
                    Ok(date) => date,
                    Err(e) => {
                        println!("Error parsing date: {:#?}", &formatted_event_date); // BU is inconsistent so can happen
                        continue;
                    }
                };
                // compare to America/New_York time using chrono_tz
                let current_date = chrono_tz::America::New_York.from_local_datetime(&chrono::Local::now().naive_local()).unwrap();
                if event_date >= current_date.date_naive() {
                    is_not_past_semester = true;
                    current_semesters.push(current_semester.clone());
                    break;
                }
            }
        }

        return current_semesters;
    }*/

    pub fn get_next_semester(&self) -> Semester {
        let mut next_year = self.semester_year;
        let next_season = match self.semester_season {
            SemesterSeason::Summer1 => SemesterSeason::Summer2,
            SemesterSeason::Summer2 => SemesterSeason::Fall,
            SemesterSeason::Fall => {
                next_year += 1;
                SemesterSeason::Spring
            },
            SemesterSeason::Spring => SemesterSeason::Summer1,
        };
        return Semester {
            semester_season: next_season,
            semester_year: next_year
        }
    }

    pub fn get_current_semester() -> Option<Semester> {
        let current_date = chrono_tz::America::New_York.from_local_datetime(&chrono::Local::now().naive_local()).unwrap();
        let semester_season = SemesterSeason::get_current_season();
        let semester_year = current_date.year() as u16;
        if semester_season.is_none() {
            return None;
        }
        return Some(Semester {
            semester_season: semester_season.unwrap(),
            semester_year
        });
    }

    pub fn get_current_or_upcoming() -> Semester {
        let current_semester = Semester::get_current_semester();
        if current_semester.is_some() {
            return current_semester.unwrap();
        } else {
            return Semester::get_next_semester(&Semester::get_next_semester(&Semester::get_current_semester().unwrap()));
        }
    }

    pub fn decode(row: &sqlx::mysql::MySqlRow) -> Result<Self, sqlx::Error> {
        let semester_season = row.try_get::<&str, &str>("semester_season")?;
        Ok(Semester {
            semester_season: SemesterSeason::from_str(semester_season).unwrap(),
            semester_year: row.try_get("semester_year")?,
        })
    }

}

impl ToString for Semester {
    fn to_string(&self) -> String {
        return String::from(self.semester_season.to_string() + " " + &self.semester_year.to_string())
    }
}


struct DayOfTheMonth {
    month: u32,
    day: u32
}

impl SemesterSeason {

    pub fn to_semester(self, year: u16) -> Semester {
        return Semester {
            semester_season: self,
            semester_year: year
        }
    }

    /// note: these may not be exact dates, but they are close enough for what we need
    fn get_season_start_end(&self) -> (DayOfTheMonth, DayOfTheMonth) {
        match self {
            SemesterSeason::Summer1 => (DayOfTheMonth { month: 5, day: 20 }, DayOfTheMonth { month: 6, day: 30 }),
            SemesterSeason::Summer2 => (DayOfTheMonth { month: 7, day: 1 }, DayOfTheMonth { month: 8, day: 9 }),
            SemesterSeason::Fall => (DayOfTheMonth { month: 9, day: 1 }, DayOfTheMonth { month: 12, day: 23 }),
            SemesterSeason::Spring => (DayOfTheMonth { month: 1, day: 20 }, DayOfTheMonth { month: 5, day: 19 }),
        }
    }

    pub fn get_current_season() -> Option<SemesterSeason> {
        let current_date = chrono_tz::America::New_York.from_local_datetime(&chrono::Local::now().naive_local()).unwrap();
        let seasons = [SemesterSeason::Summer1, SemesterSeason::Summer2, SemesterSeason::Fall, SemesterSeason::Spring];
        for season in seasons.iter() {
            let (start, end) = season.get_season_start_end();
            if current_date.month() >= start.month && current_date.month() <= end.month {
                if current_date.month() == start.month && current_date.day() < start.day {
                    continue;
                }
                if current_date.month() == end.month && current_date.day() > end.day {
                    continue;
                }
                return Some(season.to_owned());
            }
        }
        return None;
    }

    pub fn is_summer_session(&self) -> bool {
        match self {
            SemesterSeason::Summer1 => true,
            SemesterSeason::Summer2 => true,
            _ => false
        }
    }

}

impl ToString for SemesterSeason {
    fn to_string(&self) -> String {
        match self {
            SemesterSeason::Summer1 => "Summer 1".to_string(),
            SemesterSeason::Summer2 => "Summer 2".to_string(),
            SemesterSeason::Fall => "Fall".to_string(),
            SemesterSeason::Spring => "Spring".to_string(),
        }
    }
}

impl FromStr for SemesterSeason {
    type Err = SemesterParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower_season = s.replace(" ", "").to_lowercase();
        match lower_season.as_str() {
            "summer1" => Ok(SemesterSeason::Summer1),
            "summer2" => Ok(SemesterSeason::Summer2),
            "fall" => Ok(SemesterSeason::Fall),
            "spring" => Ok(SemesterSeason::Spring),
            _ => Err(SemesterParseError {
                message: (s.to_owned() + " is not a valid season name!")
            })
        }
    }
}

impl FromStr for Semester {
    type Err = SemesterParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split_whitespace().collect::<Vec<&str>>();
        let modified_str; // to increase the life of the var we fine outside if statement
        if parts.len() == 3 {
            let temp = parts.remove(0);
            modified_str = temp.to_owned() + parts[0];
            parts[0] = &modified_str;
        }
        let season = SemesterSeason::from_str(parts[0])?;
        let year = match parts[1].parse::<u16>() {
            Ok(year) => year,
            Err(_) => return Err(SemesterParseError {
                message: (parts[1].to_owned() + " is not a valid year!")
            })
        };
        return Ok(Semester {
            semester_season: season,
            semester_year: year
        })
    }
}

#[derive(Debug)]
pub struct SemesterParseError {
    message: String,
}

impl Display for SemesterParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}