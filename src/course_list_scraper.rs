use chrono::{Datelike, TimeZone};
use regex::Regex;
use scraper::{Element, ElementRef, Selector};

use crate::data_structs::bu_course::CourseSection;
use crate::data_structs::semester::{Semester, SemesterSeason};
use crate::database::DatabasePool;

const BU_URL: &str = "https://www.bu.edu";

pub async fn discover_summer_courses(database: &DatabasePool) {
    // todo get depts from here: https://www.bu.edu/summer/registration/course-codes-numbers/
    let departments = database.get_all_course_departments().await;
    for department in departments {
        for session in vec!["SUM1", "SUM2"] {
            get_summer_sites_for_department(database, session, department.clone()).await;
        }
    }
}

async fn get_summer_sites_for_department(database: &DatabasePool, summer_session: &str, department: String) {
    let div_selector: Selector = Selector::parse("div").unwrap();
    let course_list_selector: Selector = Selector::parse("li.course").unwrap();
    let course_title_selector: Selector = Selector::parse("h4.courses-name").unwrap();
    let course_code_selector: Selector = Selector::parse("p.course-id").unwrap();
    let course_info_selector: Selector = Selector::parse("p.course-info").unwrap();
    let course_term_selector: Selector = Selector::parse("p.courses-term").unwrap();
    let course_sections_selector: Selector = Selector::parse("div.section_info_container").unwrap();
    let course_sections_schedule_container: Selector = Selector::parse("div.section_schedules_container").unwrap();
    let course_section_instructor_selector: Selector = Selector::parse("div.instructor_name").unwrap();
    let course_section_notes_selector: Selector = Selector::parse("div.section_regular_notes_container").unwrap();

    let course_credit_regex = Regex::new(r"(\d+)\s*cr\.").unwrap();
    let course_term_dates_regex = Regex::new(r"\(([^)]+)\)").unwrap();

    let url = format!("/summer/courses/results.php?keywords=&session={}&time=&credits=&level=&college=&department={}&course_num=", summer_session, department);

    let result = reqwest::get(BU_URL.to_owned() + url.as_str()).await;

    let current_dt = chrono_tz::America::New_York.from_local_datetime(&chrono::Local::now().naive_local()).unwrap();
    let semester = Semester {
        semester_season: if summer_session == "SUM1" {SemesterSeason::Summer1} else {SemesterSeason::Summer2},
        semester_year: current_dt.year() as u16, //todo: not very reliable to depend on current year
    };

    let mut db_futures = Vec::new();

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
                    let instructor = section.select(&course_section_instructor_selector)
                        .next()
                        .and_then(|element| element.text().next())
                        .map(|text| text.to_string());
                    let notes = section.select(&course_section_notes_selector)
                        .next()
                        .and_then(|element| element.text().next())
                        .map(|text| text.to_string());
                    let course_section: String;
                    let course_type: Option<String>;
                    let course_schedule: Option<String>;
                    let schedule_container = section.select(&course_sections_schedule_container).next().unwrap();
                    let mut schedule_string: String = schedule_container.select(&div_selector).next().unwrap().text().next().map_or(String::new(), |s| s.to_string());
                    let mut parts = schedule_string.split_whitespace();
                    course_section = parts.next().unwrap().to_string();
                    course_type = parts.next().map(|s| s.replace("(", "").replace(")", "").to_string());
                    course_schedule = {
                        let schedule_str = parts.collect::<Vec<&str>>().join(" ");
                        if (schedule_str.is_empty()) {
                            None
                        } else {
                            Some(schedule_str)
                        }
                    };

                    let course_section = CourseSection {
                        section: course_section,
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
                let future = database.add_course(semester.clone(), course_code.to_string(), Some(course_name.to_string()), credits, true, course_sections_vec);
                db_futures.push(future);
            }
        }

        for future in db_futures {
            future.await;
        }

    } else {
        println!("Error getting summer courses for department={}", department);
    }

}

pub async fn discover_regular_semesters(database: &DatabasePool) {

    let entry_url = "/phpbin/course-search/search.php?page=w0&pagesize=1&adv=1&nolog=&search_adv_all=&yearsem_adv=*&credits=*&pathway=social&hub_match=all";

    let drop_down_selector: Selector = Selector::parse("select.coursearch-searchfields-semester-select").unwrap();

    let result = reqwest::get(BU_URL.to_owned() + entry_url).await;

    let mut target_sems: Vec<String> = Vec::new();
    if result.is_ok() {
        let result = result.unwrap();
        let text_resp = result.text().await;
        if text_resp.is_ok() {
            let text_resp = text_resp.unwrap();
            let html_document = scraper::Html::parse_document(text_resp.as_str());
            let drop_down_selection = html_document.select(&drop_down_selector).next().unwrap();
            for drop_down_item in drop_down_selection.children() {
                if !drop_down_item.value().is_element() {
                    continue;
                }
                // select the value attribute for the element
                let drop_down_entries = drop_down_item.value().as_element().unwrap().attr("value").unwrap();
                // * refers to figure semester - we don't want that, we want a current semester
                // and SUMM refers to summer session; gives incomplete info, we handle that elsewhere
                if drop_down_entries.eq("*") || drop_down_entries.contains("SUMM") {
                    continue;
                }
                target_sems.push(drop_down_entries.to_string());
            }
        }
    }

    for target_sem in target_sems {
        discover_semester_courses(&database, &target_sem).await;
    }

}

pub async fn discover_semester_courses(database: &DatabasePool, semester_key: &String) {

    let course_catalog_url: String = format!("/phpbin/course-search/search.php?page=w0&pagesize=100000&yearsem_adv={}", semester_key);

    let course_list_selector: Selector = Selector::parse("li.coursearch-result").unwrap();
    let course_heading_div_select: Selector = Selector::parse("div.coursearch-result-heading").unwrap();
    let course_description_div_select: Selector = Selector::parse("div.coursearch-result-content-description").unwrap();
    let heading_course_code_selector: Selector = Selector::parse("h6").unwrap();
    let heading_course_name_selector: Selector = Selector::parse("h2").unwrap();
    let heading_course_desc_and_credit: Selector = Selector::parse("p").unwrap();
    let course_result_sections_link: Selector = Selector::parse("a.coursearch-result-sections-link").unwrap();
    let course_section_rows_selector: Selector = Selector::parse("tr[data-section].first-row").unwrap();
    let mut course_vec = Vec::new();

    let result = reqwest::get(BU_URL.to_owned() + &course_catalog_url).await;
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
                let course_name: &str = heading_div.select(&heading_course_name_selector).next().unwrap().text().next().unwrap().trim();

                let course_credits: Option<u8> = content_div.select(&heading_course_desc_and_credit).last().map(|cred_str| {
                    cred_str.text()
                        .collect::<Vec<_>>()[0]
                        .replace("[", "")
                        .replace("cr.]", "")
                        .trim()
                        .parse::<u8>().ok()
                }).flatten();

                let opt_section_info = course.select(&course_result_sections_link).next();
                if opt_section_info.is_some() {
                    let section_info_link_tag = opt_section_info.unwrap();
                    let section_info_url = section_info_link_tag.value().attr("href").unwrap();
                    course_vec.push((course_code.to_string(), Some(course_name.to_string()), course_credits, section_info_url.to_string()));
                }
            }
        }
    }

    for (course_code, course_name, course_credits, section_info_url) in course_vec {
        let mut sections = Vec::new();
        match reqwest::get(BU_URL.to_owned() + section_info_url.as_str()).await {
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
        let semester = Semester::from_course_catalog_key(semester_key);
        database.add_course(semester, course_code, course_name, course_credits, true, sections).await;
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