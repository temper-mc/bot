use std::{
    env,
    sync::{LazyLock, OnceLock},
};

use octocrab::models::pulls::{PullRequest, Review};
use poise::serenity_prelude::prelude::SerenityError;
use poise::serenity_prelude::{ChannelId, ForumTagId, GuildId, RoleId};
use tokio::sync::{Mutex, mpsc::Sender};
use tracing::error;

pub mod commands;
pub mod pr_discussion;
pub mod webhook;
pub type CmdContext<'a> = poise::Context<'a, (), SerenityError>;

pub static TX: OnceLock<Mutex<Sender<Event>>> = OnceLock::new();

pub async fn send_event(event: Event) {
    let tx = &mut TX.get().unwrap().lock().await;
    if let Err(err) = tx.send(event).await {
        error!("Failed sending event: {err}");
    };
}

#[derive(Debug)]
pub enum Event {
    PullRequestOpened(PullRequest),
    PullRequestReady(PullRequest),
    PullRequestComment(u64, String, String),
    PullRequestApproved(PullRequest, Box<Review>),
    PullRequestMerged(PullRequest),
    PullRequestDrafted(PullRequest),
    PullRequestClosed(PullRequest),
}

#[derive(Clone)]
pub struct EnvVars {
    pub guild: GuildId,
    pub pr_channel: ChannelId,
    pub tag_draft: ForumTagId,
    pub tag_review_needed: ForumTagId,
    pub tag_approved: ForumTagId,
    pub tag_merged: ForumTagId,
    pub tag_closed: ForumTagId,
    pub member_role: RoleId,
    pub maintainer_role: RoleId,

    pub repo_owner: String,
    pub repo: String,
    pub github_token: String,
}

impl EnvVars {
    fn get(name: &str) -> u64 {
        env::var(name)
            .unwrap_or_else(|_| panic!("missing env var {name}"))
            .parse::<u64>()
            .unwrap_or_else(|_| panic!("invalid env var {name}"))
    }

    fn new() -> Self {
        Self {
            guild: GuildId::new(Self::get("GUILD")),
            pr_channel: ChannelId::new(Self::get("PR_CHANNEL")),
            tag_draft: ForumTagId::new(Self::get("FORUM_TAG_DRAFT")),
            tag_review_needed: ForumTagId::new(Self::get("FORUM_TAG_REVIEW_NEEDED")),
            tag_approved: ForumTagId::new(Self::get("FORUM_TAG_APPROVED")),
            tag_merged: ForumTagId::new(Self::get("FORUM_TAG_MERGED")),
            tag_closed: ForumTagId::new(Self::get("FORUM_TAG_CLOSED")),
            member_role: RoleId::new(Self::get("MEMBER_ROLE")),
            maintainer_role: RoleId::new(Self::get("MAINTAINER_ROLE")),

            repo_owner: env::var("REPO_OWNER").expect("missing env var REPO_OWNER"),
            repo: env::var("REPO").expect("missing env var REPO"),
            github_token: env::var("GITHUB_TOKEN").expect("missing env var GITHUB_TOKEN"),
        }
    }
}

pub static ENV_VARS: LazyLock<EnvVars> = LazyLock::new(EnvVars::new);
