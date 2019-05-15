# Activity Scraper
[My personal website](https://rymc.io/) makes use of data from my [GitHub](https://github.com/ryanmcgrath/), [Twitter](https://twitter.com/ryanmcgrath/), and [Dribbble](https://dribbble.com/ryanmcgrath) accounts. It's a static site that's republished every few minutes, at which point it scrapes the most recent activity from Twitter, various pieces of data from GitHub and Dribbble, and stores them in a file that the static engine picks up for rendering.

I figure this might be fun or useful for some other people. It's written in Rust, because... well, I enjoy writing in Rust. Nothing about this particularly requires Rust, so if you're not a fan, you may enjoy rewriting in a different language.

## Running
You need to make sure the following environment variables are set - you may want to edit the source to change the `RYMC_` prefix:

``` bash
# Dribbble
export RYMC_DRIBBBLE_API_KEY=''

# Twitter
export RYMC_TWITTER_CONSUMER_KEY=""
export RYMC_TWITTER_CONSUMER_SECRET=""
export RYMC_TWITTER_OAUTH_TOKEN=""
export RYMC_TWITTER_OAUTH_SECRET=""

# GitHub
export RYMC_GITHUB_ACCESS_TOKEN=""

export RYMC_ACTIVITY_PATH="/path/to/where/to/store"
```

Then:

``` bash
# Install Rust...
cargo build
./target/debug/social
```

## License
Do what you want with it! I make no claims of support or anything on a project like this.
