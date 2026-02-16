#![feature(str_as_str)]

use std::{env, sync::{LazyLock, OnceLock}};

use octocrab::models::pulls::PullRequest;
use poise::serenity_prelude::{ChannelId, ForumTagId, GuildId};
use poise::serenity_prelude::prelude::SerenityError;
use tokio::sync::{Mutex, mpsc::Sender};
use tracing::error;

pub mod pr_discussion;
pub mod webhook;
pub mod commands;
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
    PullRequestApproved(PullRequest),
    PullRequestMerged(PullRequest),
    PullRequestDrafted(PullRequest),
    PullRequestClosed(PullRequest),
}

#[derive(Clone, Copy)]
pub struct EnvVars {
    pub guild: GuildId,
    pub pr_channel: ChannelId,
    pub tag_draft: ForumTagId,
    pub tag_review_needed: ForumTagId,
    pub tag_approved: ForumTagId,
    pub tag_merged: ForumTagId,
    pub tag_closed: ForumTagId,
}

impl EnvVars {
    fn get(name: &str) -> u64 {
        env::var(name)
            .expect(&format!("missing env var {name}"))
            .parse::<u64>()
            .expect(&format!("invalid env var {name}"))
    }

    fn new() -> Self {
        Self {
            guild: GuildId::new(Self::get("GUILD")),
            pr_channel: ChannelId::new(Self::get("PR_CHANNEL")),
            tag_draft: ForumTagId::new(Self::get("FORUM_TAG_DRAFT")),
            tag_review_needed: ForumTagId::new(Self::get("FORUM_TAG_REVIEW_NEEDED")),
            tag_approved: ForumTagId::new(Self::get("FORUM_TAG_APPROVED")),
            tag_merged: ForumTagId::new(Self::get("FORUM_TAG_MERGED")),
            tag_closed: ForumTagId::new(Self::get("FORUM_TAG_CLOSED"))
        }
    }
}

pub static ENV_VARS: LazyLock<EnvVars> = LazyLock::new(EnvVars::new);
