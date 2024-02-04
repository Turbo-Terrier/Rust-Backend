use regex::Regex;
use scraper::{CaseSensitivity, Element, ElementRef, Selector};
use scraper::selector::CssLocalName;

use crate::data_structs::bu_course::CourseSection;
use crate::data_structs::semester::{Semester, SemesterSeason};
use crate::database::DatabasePool;

const BU_URL: &str = "https://www.bu.edu";
const COURSE_CATALOG_URL: &str = "/phpbin/course-search/search.php?page=w0&pagesize=100000&yearsem_adv=2024-SPRG";
const SUMMER_COURSE_CATALOG_URL: &str = "/summer/courses/results.php?keywords=&session=SUM1&time=&credits=&level=&college=&department=&course_num=";

// todo: finish, SUMMER_COURSE_CATALOG_URL only displays 100 courses max

pub async fn get_summer_sites(database: &DatabasePool) {
    let div_selector: Selector = Selector::parse("div").unwrap();
    let course_list_selector: Selector = Selector::parse("li.course").unwrap();
    let course_title_selector: Selector = Selector::parse("h4.courses-name").unwrap();
    let course_code_selector: Selector = Selector::parse("p.course-id").unwrap();
    let course_info_selector: Selector = Selector::parse("p.course-info").unwrap();
    let course_term_selector: Selector = Selector::parse("p.courses-term").unwrap();
    let course_sections_selector: Selector = Selector::parse("div.sections").unwrap();
    let course_sections_schedule_container: Selector = Selector::parse("div.section_schedules_container").unwrap();
    let course_section_instructor_selector: Selector = Selector::parse("div.instructor_name").unwrap();
    let course_section_notes_selector: Selector = Selector::parse("div.section_regular_notes_container").unwrap();

    let course_credit_regex = Regex::new(r"(\d+)\s*cr\.").unwrap();
    let course_term_dates_regex = Regex::new(r"\(([^)]+)\)").unwrap();

    let result = reqwest::get(BU_URL.to_owned() + SUMMER_COURSE_CATALOG_URL).await;
    if result.is_ok() {
        let result = result.unwrap();
        let text_resp = result.text().await;
        if text_resp.is_ok() {
            let text_resp = text_resp.unwrap();
            let html_document = scraper::Html::parse_document(text_resp.as_str());
            let course_iter = html_document.select(&course_list_selector).into_iter();
            for course in course_iter {
                let course_code: &str = course.select(&course_code_selector).next().unwrap().text().next().unwrap();
                let course_name: &str = course.select(&course_title_selector).next().unwrap().text().next().unwrap();
                let course_info: &str = course.select(&course_info_selector).next().unwrap().text().next().unwrap();
                let credits: Option<u8> = course_credit_regex
                    .captures(course_info)
                    .and_then(|captures| captures.get(1))
                    .and_then(|credits| credits.as_str().parse().ok());
                let course_term_raw_string: &str = course.select(&course_term_selector).next().unwrap().text().next().unwrap();
                let course_dates: Option<String> = course_term_dates_regex
                    .captures(course_term_raw_string)
                    .and_then(|captures| captures.get(1))
                    .and_then(|credits| credits.as_str().parse().ok());

                let sections = course.select(&course_sections_selector).into_iter();
                let mut course_sections_vec = Vec::new();

                for section in sections {
                    let instructor = course.select(&course_section_instructor_selector)
                        .next()
                        .and_then(|element| element.text().next())
                        .map(|text| text.to_string());
                    let notes = course.select(&course_section_notes_selector)
                        .next()
                        .and_then(|element| element.text().next())
                        .map(|text| text.to_string());
                    let section: String;
                    let course_type: Option<String>;
                    let course_schedule: Option<String>;
                    let schedule_container = course.select(&course_sections_schedule_container).next().unwrap();
                    let mut schedule_string: String = schedule_container.select(&div_selector).next().unwrap().text().next().map_or(String::new(), |s| s.to_string());
                    let mut parts = schedule_string.split_whitespace();
                    section = parts.next().unwrap().to_string();
                    course_type = parts.next().map(|s| s.trim_end().trim_start().to_string());
                    course_schedule = {
                        let schedule_str = parts.collect::<Vec<&str>>().join(" ");
                        if (schedule_str.is_empty()) {
                            None
                        } else {
                            Some(schedule_str)
                        }
                    };

                    let course_section = CourseSection{
                        section,
                        open_seats: None,
                        instructor,
                        section_type: course_type,
                        location: None,  //todo
                        schedule: course_schedule,
                        dates: course_dates.clone(),
                        notes,
                    };
                    course_sections_vec.push(course_section);
                }

                // todo hard coded rn
                let semester = Semester {
                    semester_season: SemesterSeason::Summer1,
                    semester_year: 2024,
                };
                database.add_course(&semester, course_code, Some(course_name.to_string()), credits, true, course_sections_vec).await;
            }
        }
    }
}

pub async fn get_sites(database: &DatabasePool) {

    let course_list_selector: Selector = Selector::parse("li.coursearch-result").unwrap();
    let course_heading_div_select: Selector = Selector::parse("div.coursearch-result-heading").unwrap();
    let course_description_div_select: Selector = Selector::parse("div.coursearch-result-content-description").unwrap();
    let heading_course_code_selector: Selector = Selector::parse("h6").unwrap();
    let heading_course_name_selector: Selector = Selector::parse("h2").unwrap();
    let heading_course_desc_and_credit: Selector = Selector::parse("p").unwrap();
    let course_result_sections_link: Selector = Selector::parse("a.coursearch-result-sections-link").unwrap();
    let course_section_rows_selector: Selector = Selector::parse("tr[data-section].first-row").unwrap();

    let result = reqwest::get(BU_URL.to_owned() + COURSE_CATALOG_URL).await;
    if result.is_ok() {
        let result = result.unwrap();
        let text_resp = result.text().await;
        if text_resp.is_ok() {
            let text_resp = text_resp.unwrap();
            let html_document = scraper::Html::parse_document(text_resp.as_str());
            let course_iter = html_document.select(&course_list_selector).into_iter();
            for course in course_iter {
                let heading_div = course.select(&course_heading_div_select).next().unwrap();
                let content_div = course.select(&course_description_div_select).next().unwrap();
                let course_code: &str = heading_div.select(&heading_course_code_selector).next().unwrap().text().next().unwrap();
                let course_name: &str = heading_div.select(&heading_course_name_selector).next().unwrap().text().next().unwrap();

                let course_credits: Option<u8> = content_div.select(&heading_course_desc_and_credit).last().map(|cred_str| {
                    cred_str.text()
                        .collect::<Vec<_>>()[0]
                        .replace("[", "")
                        .replace("cr.]", "")
                        .trim()
                        .parse::<u8>().ok()
                }).flatten();

                let mut sections = Vec::new();
                let opt_section_info = course.select(&course_result_sections_link).next();
                if opt_section_info.is_some() {
                    let section_info_link_tag = opt_section_info.unwrap();
                    let section_info_url = section_info_link_tag.value().attr("href").unwrap();
                    match reqwest::get(BU_URL.to_owned() + section_info_url).await {
                        Ok(resp) => {
                            let raw_sections_html = resp.text().await.unwrap();
                            let sections_html_document = scraper::Html::parse_document(raw_sections_html.as_str());
                            for a_row in sections_html_document.select(&course_section_rows_selector) {
                                sections.push(process_section_row(a_row));
                            }
                        },
                        Err(e) => {
                            println!("Error getting section info for {}: {}", course_code, e);
                        }
                    };
                }
                // todo hard coded rn
                let semester = Semester {
                    semester_season: SemesterSeason::Spring,
                    semester_year: 2024,
                };
                database.add_course(&semester, course_code, Some(course_name.to_string()), course_credits, true, sections).await;
            }
        }
    }
}

fn process_section_row(a_row: ElementRef) -> CourseSection {

    let mut section = String::from("A1");
    let mut open_seats: Option<u8> = None;
    let mut instructor = None;
    let mut section_type = None;
    let mut location = None;
    let mut schedule = None;
    let mut dates = None;
    let mut notes = None;

    for (i, col) in a_row.children().into_iter().enumerate() {
        if col.value().is_element() {
            if let Some(col_val) = col.children().next() {
                if col_val.value().is_text() {
                    let text = col_val.value().as_text().map(|t| t.text.trim().to_string());
                    match i {
                        0 => section = text.unwrap_or(String::from("A1")),
                        1 => open_seats = text.and_then(|t| t.parse().ok()),
                        2 => instructor = text,
                        3 => section_type = text,
                        4 => location = text,
                        5 => schedule = text,
                        6 => dates = text,
                        7 => notes = text,
                        _ => eprintln!("Unknown column index: {}", i),
                    }
                }
            }
        }
    }


    CourseSection {
        section,
        open_seats,
        instructor,
        section_type,
        location,
        schedule,
        dates,
        notes,
    }

}