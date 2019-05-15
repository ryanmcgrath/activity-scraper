//! social/main.rs
//!
//! Implements a basic scraper + data massaging routine for social
//! activity stuff.
//!
//! @author Ryan McGrath <ryan@rymc.io>
//! @copyright RYMC 2019

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate serde_derive;

pub mod twitter;
pub mod github;
pub mod dribbble;

use chrono::{NaiveDateTime, Utc};
use chrono_humanize::{HumanTime, Accuracy, Tense};
use serde::Serializer;

#[derive(Serialize, Debug)]
pub struct DateTime {
    pub url: String,
    pub action: String,

    #[serde(serialize_with = "serialize_timestamp")]
    pub ts: chrono::NaiveDateTime
}

#[derive(Serialize, Debug)]
pub struct Activity {
    #[serde(rename = "type")]
    pub activity_type: String,
    pub content: String,
    pub datetime: DateTime
}

impl Activity {
    pub fn new(activity_type: &str, content: String, datetime: DateTime) -> Self {
        Activity {
            activity_type: activity_type.to_string(),
            content: content,
            datetime: datetime
        }
    }
}

//const FORMAT: &'static str = "%a %d %B %Y ~%R";
fn serialize_timestamp<S>(dt: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    //let s = format!("{}", dt.format(FORMAT));
    let now = Utc::now().naive_utc();
    let duration = dt.signed_duration_since(now);
    let s = HumanTime::from(duration);
    serializer.serialize_str(&s.to_text_en(Accuracy::Rough, Tense::Past))
}

pub fn markdown_link_title_escape(s: &str) -> String {
    s.replace("\"", "&#34;").replace("(", "&#40;").replace(")", "&#41;")
}

fn main() {
    dotenv::dotenv().ok();
    let mut feed: Vec<Activity> = vec![];

    match twitter::get_and_transform_tweets_to_html() {
        Ok(mut tweets) => { feed.append(&mut tweets); },
        Err(e) => { eprintln!("Error fetching Tweets: {:?}", e); }
    }

    match github::get_and_transform_activity_to_html() {
        Ok(mut activity) => { feed.append(&mut activity); },
        Err(e) => { eprintln!("Error fetching GitHub Activity: {:?}", e); }
    }
    
    match dribbble::get_and_transform_activity_to_html() {
        Ok(mut activity) => { feed.append(&mut activity); },
        Err(e) => { eprintln!("Error fetching Dribbble Shots: {:?}", e); }
    }

    feed.sort_by(|a, b| {
        b.datetime.ts.cmp(&a.datetime.ts)
    });
    
    let path = std::env::var("RYMC_ACTIVITY_PATH").expect("Activity feed filepath not set!");
    let contents = serde_json::to_string(&feed[0..12]).expect("Unable to serialize Feed JSON! :(");
    std::fs::write(&format!("{}/activities.json", path), contents).expect("Could not write activity feed to file!");
}
