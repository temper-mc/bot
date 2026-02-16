use std::sync::Arc;

use octocrab::models::pulls::PullRequest;
use poise::serenity_prelude::{ChannelId, Context, CreateForumPost, CreateMessage, EditThread, ForumTagId};
use tracing::error;

use crate::ENV_VARS;

pub async fn pr_created(ctx: &Arc<Context>, pr: PullRequest) {
    let name = format!("#{} - {} by {}", pr.number, pr.title.unwrap_or("Unnamed".to_string()), pr.user.map(|u| u.login).unwrap_or("Unknown".to_string()));

    let url = pr.html_url.expect("PR missing HTML URL?").as_str().to_string();
    let tag = if pr.draft.unwrap_or_default() { ENV_VARS.tag_draft } else { ENV_VARS.tag_review_needed };
    let post = CreateForumPost::new(name, CreateMessage::new().content(url)).add_applied_tag(tag);
    
    if let Err(err) = ENV_VARS.pr_channel.create_forum_post(ctx, post).await {
        error!("Failed creating PR forum post: {err}");
    };
}

pub async fn apply_tag(ctx: &Arc<Context>, id: u64, tag: ForumTagId) {
    let Some(channel) = find_pr_post(ctx, id).await else {
        error!("Missing forum post for PR #{id}");
        return;
    };
    
    let edit = EditThread::new().applied_tags(vec![tag]);
    if let Err(err) = channel.edit_thread(ctx, edit).await {
        error!("Failed editing thread for PR #{id}: {err}");
    }
}

pub async fn send_message(ctx: &Arc<Context>, id: u64, message: CreateMessage) {
    let Some(channel) = find_pr_post(ctx, id).await else {
        error!("Missing forum post for PR #{id}");
        return;
    };
    
    if let Err(err) = channel.send_message(ctx, message).await {
        error!("Failed sending message for PR #{id}: {err}");
    }
}

async fn find_pr_post(ctx: &Arc<Context>, id: u64) -> Option<ChannelId> {
    let threads = match ENV_VARS.guild.get_active_threads(ctx).await {
        Ok(data) => data,
        Err(err) => {
            error!("Failed fetching active threads: {err}");
            return None
        }
    };
    
    for thread in threads.threads {
        if !thread.parent_id.is_some_and(|id| id == ENV_VARS.pr_channel) {
            continue
        }
        
        if !thread.name().starts_with(&format!("#{id}")) {
            continue
        }
        
        return Some(thread.id)
    }
    
    None
}
