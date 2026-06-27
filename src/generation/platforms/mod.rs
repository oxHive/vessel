pub mod twitter;
pub mod linkedin;
pub mod bluesky;
pub mod mastodon;
pub mod discord;
pub mod github_release;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Twitter,
    LinkedIn,
    Bluesky,
    Mastodon,
    Discord,
    GitHubRelease,
}

pub struct PlatformSpec {
    pub name: &'static str,
    pub char_limit: Option<usize>,
    pub tone_guidance: &'static str,
    pub format_notes: &'static str,
    pub hashtag_notes: &'static str,
}

impl Platform {
    pub fn all_v1() -> Vec<Platform> {
        vec![
            Platform::Twitter,
            Platform::LinkedIn,
            Platform::Bluesky,
            Platform::Mastodon,
            Platform::Discord,
            Platform::GitHubRelease,
        ]
    }

    pub fn spec(&self) -> PlatformSpec {
        match self {
            Platform::Twitter => twitter::spec(),
            Platform::LinkedIn => linkedin::spec(),
            Platform::Bluesky => bluesky::spec(),
            Platform::Mastodon => mastodon::spec(),
            Platform::Discord => discord::spec(),
            Platform::GitHubRelease => github_release::spec(),
        }
    }

    pub fn validate_length(&self, content: &str) -> Option<usize> {
        let spec = self.spec();
        spec.char_limit.and_then(|limit| {
            let len = content.chars().count();
            if len > limit {
                Some(len - limit)
            } else {
                None
            }
        })
    }

    pub fn slug(&self) -> &'static str {
        match self {
            Platform::Twitter => "twitter",
            Platform::LinkedIn => "linkedin",
            Platform::Bluesky => "bluesky",
            Platform::Mastodon => "mastodon",
            Platform::Discord => "discord",
            Platform::GitHubRelease => "github_release",
        }
    }
}
