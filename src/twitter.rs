//! twitter.rs
//!
//! Handles fetching and loading Tweets.
//! 
//! @author Ryan McGrath <ryan@rymc.io>
//! @copyright RYMC 2019

use std::env::var;
use std::error::Error;

use serde::{Deserialize, Deserializer};
use chrono::NaiveDateTime;
use oauth_client::{get, Token, ParamList};

use crate::{Activity, DateTime, markdown_link_title_escape};

#[derive(Deserialize, Debug)]
pub struct Url {
    pub url: String,
    pub display_url: String,
    pub expanded_url: String,
    pub indices: [u32; 2]
}

#[derive(Deserialize, Debug)]
pub struct HashTag {
    pub text: String,
    pub indices: [u32; 2]
}

#[derive(Deserialize, Debug)]
pub struct UserMention {
    pub screen_name: String,
    pub id_str: String,
    pub indices: [u32; 2]
}

#[derive(Deserialize, Debug)]
pub struct Media {
    pub id_str: String,
    pub url: String,
    pub display_url: String,
    pub expanded_url: String
}

#[derive(Deserialize, Debug)]
pub struct ExtendedEntities {
    pub media: Vec<Media>
}

#[derive(Deserialize, Debug)]
pub struct Entities {
    pub hashtags: Vec<HashTag>,
    pub user_mentions: Vec<UserMention>,
    pub urls: Vec<Url>,
    pub media: Option<Vec<Media>>
}

#[derive(Deserialize, Debug)]
pub struct User {
    pub screen_name: String
}

#[derive(Deserialize, Debug)]
pub struct Tweet {
    pub id_str: String,
    pub full_text: String,
    pub lang: String,
    pub user: User,
    pub entities: Entities,
    pub extended_entities: Option<ExtendedEntities>,
    pub retweeted_status: Option<serde_json::Value>,

    #[serde(deserialize_with = "parse_twitter_dt")]
    pub created_at: NaiveDateTime
}

fn parse_twitter_dt<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error> where D: Deserializer<'de> {
    let s = String::deserialize(deserializer)?;
    NaiveDateTime::parse_from_str(&s, "%a %b %d %H:%M:%S %z %Y").map_err(serde::de::Error::custom)
}

fn patch_text(mut text: String, tweet: &Tweet) -> String {
    // RTs get some entities of their own, so we'll recurse slightly to cover them.
    if let Some(retweeted_status) = &tweet.retweeted_status {
        if let Ok(retweet) = serde_json::from_value::<Tweet>(retweeted_status.clone()) {
            return format!(
                "RT [@{}](https://twitter.com/{} \"View {} on Twitter\") {}",
                retweet.user.screen_name, retweet.user.screen_name,
                markdown_link_title_escape(&retweet.user.screen_name),
                patch_text(retweet.full_text.clone(), &retweet)
            );
        }
    }

    for mention in tweet.entities.user_mentions.iter() {
        let x = format!("@{}", mention.screen_name);
        text = text.replace(&x, &format!(
            "[@{}](https://twitter.com/{} \"View @{} on Twitter\")",
            mention.screen_name, mention.screen_name,
            markdown_link_title_escape(&mention.screen_name)
        ));
    }

    for hashtag in tweet.entities.hashtags.iter() {
        let x = format!("#{}", hashtag.text);
        text = text.replace(&x, &format!(
            "[#{}](https://twitter.com/hashtag/{} \"View #{} on Twitter\")",
            hashtag.text, hashtag.text, markdown_link_title_escape(&hashtag.text)
        ));
    }

    for url in tweet.entities.urls.iter() {
        text = text.replace(&url.url, &format!(
            "[{}]({})",
            url.display_url, url.expanded_url
        ));
    }

    if let Some(media_entities) = &tweet.entities.media {
        for media in media_entities.iter() {
            text = text.replace(&media.url, "");
        }
    }

    // Native media only exists if it's actually native. It's weird, but we'll just replace it with
    // the nicer URL for now... maybe down the road we'll auto-load images or something.
    if let Some(entities) = &tweet.extended_entities {
        for media in entities.media.iter() {
            text = text.replace(&media.url, &format!(
                "[https://{}](https://{})",
                media.display_url, media.display_url
            ));
        }
    }

    text
}

/// Calls out to Twitter and retrieves Tweets, then pushes them into a standard
/// template that'll ultimately be rendered on the HTML side.
pub fn get_and_transform_tweets_to_html() -> Result<Vec<Activity>, Box<Error>> {
    let endpoint = "https://api.twitter.com/1.1/statuses/user_timeline.json";
    let consumer = Token::new(var("RYMC_TWITTER_CONSUMER_KEY")?, var("RYMC_TWITTER_CONSUMER_SECRET")?);
    let access = Token::new(var("RYMC_TWITTER_OAUTH_TOKEN")?, var("RYMC_TWITTER_OAUTH_SECRET")?);

    let mut options = ParamList::new();
    options.insert("tweet_mode".into(), "extended".into());
    options.insert("count".into(), "10".into());
    options.insert("screen_name".into(), "ryanmcgrath".into());

    let bytes = get(endpoint, &consumer, Some(&access), Some(&options))?;
    let response = String::from_utf8(bytes)?;
    let mut tweets: Vec<Tweet> = serde_json::from_str(&response)?;

    let mut activities: Vec<Activity> = vec![];
    for tweet in tweets.iter_mut() {
        activities.push(Activity::new("twitter", patch_text(tweet.full_text.clone(), &tweet), DateTime {
            action: "Tweeted".into(),
            url: format!("https://twitter.com/ryanmcgrath/status/{}", tweet.id_str),
            ts: tweet.created_at
        }));
    }

    Ok(activities)
}
