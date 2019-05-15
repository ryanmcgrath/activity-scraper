//! dribbble.rs
//!
//! Queries Dribbble's API and gets the shots for my profile.
//! If a shot is recent enough it'll also display as an Activity
//! on the homepage.
//!
//! @author Ryan McGrath <ryan@rymc.io>
//! @copyright RYMC 2019

use std::env::var;
use std::error::Error;
use serde::{Deserializer, Deserialize};
use chrono::NaiveDateTime;

use crate::{Activity, DateTime, markdown_link_title_escape};

#[derive(Deserialize, Debug)]
pub struct ImageSet {
    pub hidpi: Option<String>,
    pub normal: String,
    pub teaser: String
}

#[derive(Deserialize, Debug)]
pub struct Shot {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub images: ImageSet,
    pub html_url: String,
    pub width: i32,
    pub height: i32,
    pub tags: Vec<String>,

    #[serde(deserialize_with = "deserialize_dribbble_timestamp")]
    pub published_at: NaiveDateTime,
    
    #[serde(deserialize_with = "deserialize_dribbble_timestamp")]
    pub updated_at: NaiveDateTime
}

const FORMAT: &'static str = "%Y-%m-%dT%H:%M:%SZ";
fn deserialize_dribbble_timestamp<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error> where D: Deserializer<'de> {
    let s = String::deserialize(deserializer)?;
    NaiveDateTime::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
}

pub fn get_and_transform_activity_to_html() -> Result<Vec<Activity>, Box<Error>> {
    let access_token = var("RYMC_DRIBBBLE_API_KEY")?;
    let endpoint = format!("https://api.dribbble.com/v2/user/shots?access_token={}", access_token);
    let response = reqwest::get(&endpoint)?.text()?;
    let shots: Vec<Shot> = serde_json::from_str(&response)?;

    // Write it ahead of time, as the Designs tab also uses this data
    let path = std::env::var("RYMC_ACTIVITY_PATH")?;
    std::fs::write(&format!("{}/dribbble.json", path), response)?;

    let mut activities: Vec<Activity> = vec![];
    for shot in shots {
        let tags: Vec<String> = shot.tags.iter().map(|tag| format!(
            "[#{}](https://dribbble.com/ryanmcgrath/tags/{} \"View shots tagged {} on Dribbble\")",
            tag, tag, markdown_link_title_escape(&tag)
        )).collect();
        
        let content = format!(
            "Unveiled a new Shot: [{}]({} \"View {} on Dribbble\") [![{}]({})]({} \"View {} on Dribbble\")\n\n{}",
            shot.title, shot.html_url, markdown_link_title_escape(&shot.title),
            shot.title, shot.images.teaser, shot.html_url, markdown_link_title_escape(&shot.title), tags.join(" ")
        );
        
        activities.push(Activity::new("dribbble", content, DateTime {
            action: "Shot".into(),
            url: shot.html_url,
            ts: shot.published_at
        }));
    }

    Ok(activities)
}
