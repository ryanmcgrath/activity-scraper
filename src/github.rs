//! github.rs
//!
//! Handles fetching and loading GitHub activity.
//! 
//! @author Ryan McGrath <ryan@rymc.io>
//! @copyright RYMC 2019

use std::{env::var, error::Error, fmt};
use serde::{Deserializer, Deserialize};
use chrono::NaiveDateTime;
use linkify::LinkFinder;
use regex::Regex;

use crate::{Activity, DateTime, markdown_link_title_escape};

lazy_static! {
    static ref SOCIAL_MENTION_REGEX: Regex = Regex::new(r"(@[\w_-]+)").unwrap();
    static ref SOCIAL_HASHTAG_REGEX: Regex = Regex::new(r"(@[\w_-]+)").unwrap();
}

#[derive(Debug)]
pub struct GHKeyError {
    keypath: String
}

impl Error for GHKeyError {}

impl fmt::Display for GHKeyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Invalid key supplied ({}). Ignoring!", self.keypath)
    }
}

impl GHKeyError {
    pub fn raise(keypath: &str) -> Result<String, Box<Error>> {
        Err(Box::new(GHKeyError {
            keypath: keypath.into()
        }))
    }
}

fn get(value: &serde_json::Value, path: &str) -> Result<String, Box<Error>> {
    let keys: Vec<String> = path.split(".").map(|s| s.to_owned()).collect();
    let mut v = value;

    for key in keys {
        v = v.get(&key).ok_or_else(|| GHKeyError {
            keypath: key
        })?;
    }

    Ok(v.as_str().ok_or_else(|| GHKeyError {
        keypath: path.into()
    })?.to_string())
}

const FORMAT: &'static str = "%Y-%m-%dT%H:%M:%SZ";
fn deserialize_github_timestamp<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error> where D: Deserializer<'de> {
    let s = String::deserialize(deserializer)?;
    NaiveDateTime::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
}

#[derive(Deserialize, Debug)]
pub struct Repository {
    name: String,
    url: String
}

#[derive(Deserialize, Debug)]
pub struct GitHubActivity {
    #[serde(rename = "type")]
    pub action: String,

    pub repo: Repository,
    pub payload: serde_json::Value,

    #[serde(deserialize_with = "deserialize_github_timestamp")]
    pub created_at: NaiveDateTime
}

fn clean_text(s: &str) -> String {
    let cut: Vec<String> = s.to_string().split("\n\n> On").map(|x| {
        x.to_owned()
    }).collect();
    
    let mut text = cut[0].to_string();
        
    let link_finder = LinkFinder::new();
    let links: Vec<_> = link_finder.links(s).collect();

    // For each link, markdown-ify it
    for link in links {
        // If this exists, it's high chances it's already markdown
        // e.g, ](http...
        let start = link.start() - 2;
        if start > 0 && text.chars().nth(start) == Some(']') {
            continue;
        }

        let l = link.as_str();
        text = text.replace(link.as_str(), &format!("[{}]({})", l, l));
    }
    
    let mentions: Vec<String> = SOCIAL_MENTION_REGEX.captures_iter(&text).map(|capture| {
        capture.get(0).unwrap().as_str().to_owned()
    }).collect();
    
    for mention in mentions {
        text = text.replace(&mention, &format!(
            "[{}]({})", mention,
            format!("https://github.com/{}", mention.replace("@", ""))
        ));
    }

    let hashtags: Vec<String> = SOCIAL_HASHTAG_REGEX.captures_iter(&text).map(|capture| {
        capture.get(0).unwrap().as_str().to_owned()
    }).collect();
    
    // A hashtag in GitHub refers to an issue, potentially in another repo!
    //for hashtag in hashtags {
        /*text = text.replace(&hashtag, &format!(
            "[{}]({})", hashtag,
            format!("https://github.com/{}", hashtag.replace("#", ""))
        ));*/
    //}

    text
}

fn patch_text(activity: &GitHubActivity) -> Result<String, Box<Error>> { match activity.action.as_ref() {
    "CommitCommentEvent" => { Ok(format!(
        "{} on [{}]({} \"View {} on GitHub\")",
        clean_text(&get(&activity.payload, "comment.body")?),
        activity.repo.name, activity.repo.url, markdown_link_title_escape(&activity.repo.name)
    ))},
    
    "IssueCommentEvent" => {
        let action = &get(&activity.payload, "action")?;
        if action != "created" { GHKeyError::raise("IssueCommentEvent.payload.action")?; }

        let title = get(&activity.payload, "issue.title")?;
        let body = get(&activity.payload, "comment.body")?;

        Ok(format!(
            "{} on [{}]({} \"View {} on GitHub\")",
            clean_text(&body), title,
            get(&activity.payload, "issue.html_url")?,
            markdown_link_title_escape(&title)
        ))
    },
    
    "ForkEvent" => {
        let full_name = get(&activity.payload, "forkee.full_name")?;

        Ok(format!(
            "Forked [@{}](https://github.com/{} \"View {} on GitHub\") to [@{}]({} \"View {} on GitHub\")",
            activity.repo.name, activity.repo.name, activity.repo.name,
            full_name,
            get(&activity.payload, "forkee.html_url")?,
            markdown_link_title_escape(&full_name)
        ))
    },

    "CreateEvent" =>{ match get(&activity.payload, "ref_type")?.as_ref() {
        "repository" => {
            let full_name = &activity.repo.name;

            Ok(format!(
                "Created [@{}](https://github.com/{} \"View {} on GitHub\")",
                full_name, full_name, markdown_link_title_escape(&full_name)
            ))
        },

        _ => GHKeyError::raise("CreateEvent.ref_type")
    }},

    
    "IssuesEvent" => { match get(&activity.payload, "action")?.as_ref() {
        "opened" => {
            let title = get(&activity.payload, "issue.title")?;
            let repo = get(&activity.payload, "repository.full_name")?;

            Ok(format!(
                "Opened [{}]({} \"View {} on GitHub\") in [@{}](https://github.com/{} \"View {} on GitHub\")",
                title, get(&activity.payload, "issue.html_url")?,
                markdown_link_title_escape(&title), repo, repo, markdown_link_title_escape(&repo)
            ))
        },
        
        "closed" => {
            let title = get(&activity.payload, "issue.title")?;
            let repo = &activity.repo.name;

            Ok(format!(
                "Closed [{}]({} \"View {} on GitHub\") in [@{}](https://github.com/{} \"View {} on GitHub\")",
                title, get(&activity.payload, "issue.html_url")?,
                markdown_link_title_escape(&title), repo, repo, markdown_link_title_escape(&repo)
            ))
        },

        _ => GHKeyError::raise("IssuesEvent.action")
    }},
   
    "PullRequestEvent" => { match get(&activity.payload, "action")?.as_ref() {
        "opened" => {
            let full_name = get(&activity.payload, "pull_request.base.repo.full_name")?;

            Ok(format!(
                "Opened a pull request in [@{}](https://github.com/{} \"View {} on GitHub\"):\n\n[{}]({} \"View this PR on GitHub\")",
                full_name, full_name, full_name,
                get(&activity.payload, "pull_request.title")?,
                get(&activity.payload, "pull_request.html_url")?
            ))
        },

        "closed" => {
            let full_name = get(&activity.payload, "pull_request.base.repo.full_name")?;
            
            Ok(format!(
                "Closed a pull request in [@{}](https://github.com/{} \"View {} on GitHub\"):\n\n[{}]({} \"View this PR on GitHub\")",
                full_name, full_name, full_name,
                get(&activity.payload, "pull_request.title")?,
                get(&activity.payload, "pull_request.html_url")?
            ))
        },

        _ => GHKeyError::raise("PullRequestEvent.action")
    }},
    
    "PushEvent" => {
        let no = activity.payload.get("distinct_size").ok_or_else(|| GHKeyError {
            keypath: "PushEvent.distinct_size".into()
        })?.as_i64().ok_or_else(|| GHKeyError {
            keypath: "PushEvent.distinct_size.parser_error".into()
        })?;

        let compare_url = format!(
            "https://github.com/{}/compare/{}...{}",
            activity.repo.name,
            get(&activity.payload, "before")?,
            get(&activity.payload, "head")?
        );

        Ok(format!(
            "Pushed [{} commit{}]({} \"View these changes on GitHub\") to [@{}](https://github.com/{} \"View {} on GitHub\")",
            no, match no {
                1 => "",
                _ => "s"
            }, compare_url, activity.repo.name,
            activity.repo.name, markdown_link_title_escape(&activity.repo.name)
        ))
    },
    
    "PublicEvent" => {
        let full_name = get(&activity.payload, "repository.full_name")?;

        Ok(format!(
            "Open sourced [@{}](https://github.com/{} \"View {} on GitHub\")",
            full_name, full_name, markdown_link_title_escape(&full_name)
        ))
    },
        
    "ReleaseEvent" => {
        Ok(format!(
            "Released [@{} {}]({} \"View this release on GitHub\")",
            get(&activity.payload, "repository.full_name")?,
            get(&activity.payload, "release.tag_name")?,
            get(&activity.payload, "release.html_url")?
        ))
    },

    uncaught => GHKeyError::raise(uncaught)
}}

pub fn get_and_transform_activity_to_html() -> Result<Vec<Activity>, Box<Error>> {
    let access_token = var("RYMC_GITHUB_ACCESS_TOKEN").expect("GITHUB_ACCESS_TOKEN not set!");

    // Fetch the repositories, which the Code tab uses for UI. Then we'll grab activity to render
    // in the sidebar.
    let repositories_endpoint = format!("https://api.github.com/users/ryanmcgrath/repos?access_token={}&sort=pushed", access_token);
    let repositories = reqwest::get(&repositories_endpoint)?.text()?;
    let path = std::env::var("RYMC_ACTIVITY_PATH").expect("Activity feed filepath not set!");
    std::fs::write(&format!("{}/github-repos.json", path), repositories).expect("Could not write Dribbble shots to file!");

    // Now we can do our normal thing - fetch activity and render Markdown/etc.
    let activities_endpoint = format!("https://api.github.com/users/ryanmcgrath/events/public?access_token={}", access_token);
    let github_activities: Vec<GitHubActivity> = reqwest::get(&activities_endpoint)?.json()?;

    let mut activities: Vec<Activity> = vec![];
    for activity in github_activities {
        let content = match patch_text(&activity) {
            Ok(c) => c,
            Err(e) => { eprintln!("{}", e); continue; }
        };

        activities.push(Activity::new("github", content, DateTime {
            action: "On".into(),
            url: "".into(),
            ts: activity.created_at
        }));
    }

    Ok(activities)
}
