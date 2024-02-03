use scraper::{ElementRef, Selector};

use crate::data_structs::bu_course::CourseSection;
use crate::data_structs::semester::{Semester, SemesterSeason};
use crate::database::DatabasePool;

const BU_URL: &str = "https://www.bu.edu";
const COURSE_CATALOG_URL: &str = "/phpbin/course-search/search.php?page=w0&pagesize=100000&yearsem_adv=2024-SPRG";

pub async fn get_sites(database: &DatabasePool, search_sections_for_existing_courses: bool) {

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