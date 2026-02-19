use std::{env, sync::Arc};

use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::Request,
    routing::post,
};
use octocrab::models::{
    AuthorAssociation,
    pulls::ReviewState,
    webhook_events::{
        WebhookEvent, WebhookEventPayload, WebhookEventType,
        payload::{
            PullRequestReviewCommentWebhookEventAction, PullRequestReviewWebhookEventAction,
            PullRequestReviewWebhookEventPayload, PullRequestWebhookEventAction,
            PullRequestWebhookEventPayload,
        },
    },
};
use tokio::net::TcpListener;
use tracing::{error, info, trace};

use crate::{Event, send_event};

struct Secret(String);

async fn handle_pr_created(event: Box<PullRequestWebhookEventPayload>) {
    let pr = event.pull_request;
    info!(
        "PR created: #{} - {:?} by {:?}",
        event.number,
        pr.clone().title,
        pr.clone().user.map(|u| u.login)
    );
    send_event(Event::PullRequestOpened(pr)).await;
}

async fn handle_pr_ready(event: Box<PullRequestWebhookEventPayload>) {
    let pr = event.pull_request;
    info!(
        "PR ready for review: #{} - {:?} by {:?}",
        event.number,
        pr.clone().title,
        pr.clone().user.map(|u| u.login)
    );
    send_event(Event::PullRequestReady(pr)).await;
}

async fn handle_pr_closed(event: Box<PullRequestWebhookEventPayload>) {
    let pr = event.pull_request;
    if !pr.merged.unwrap_or_default() {
        info!(
            "PR closed: #{} - {:?} by {:?}",
            event.number,
            pr.clone().title,
            pr.clone().user.map(|u| u.login)
        );
        send_event(Event::PullRequestClosed(pr)).await;
    } else {
        info!(
            "PR merged: #{} - {:?} by {:?}",
            event.number,
            pr.clone().title,
            pr.clone().user.map(|u| u.login)
        );
        send_event(Event::PullRequestMerged(pr)).await;
    }
}

async fn handle_pr_drafted(event: Box<PullRequestWebhookEventPayload>) {
    let pr = event.pull_request;
    info!(
        "PR drafted: #{} - {:?} by {:?}",
        event.number,
        pr.clone().title,
        pr.clone().user.map(|u| u.login)
    );
    send_event(Event::PullRequestDrafted(pr)).await;
}

async fn handle_pr_reopened(event: Box<PullRequestWebhookEventPayload>) {
    let pr = event.pull_request;
    info!(
        "PR reopened: #{} - {:?} by {:?}",
        event.number,
        pr.clone().title,
        pr.clone().user.map(|u| u.login)
    );

    if pr.draft.unwrap_or_default() {
        send_event(Event::PullRequestDrafted(pr)).await;
    } else {
        send_event(Event::PullRequestReady(pr)).await;
    }
}

async fn handle_pr_approved(event: Box<PullRequestReviewWebhookEventPayload>) {
    let pr = event.pull_request;
    let review = event.review;

    let approved = review
        .state
        .is_some_and(|state| state == ReviewState::Approved);
    let is_contrib = matches!(
        review
            .clone()
            .author_association
            .unwrap_or(AuthorAssociation::None),
        AuthorAssociation::Collaborator
            | AuthorAssociation::Contributor
            | AuthorAssociation::Member
            | AuthorAssociation::Owner
    );

    if approved && is_contrib {
        info!(
            "PR approved by {:?}: #{} - {:?} by {:?}",
            review.clone().user.map(|user| user.login),
            pr.number,
            pr.clone().title,
            pr.clone().user.map(|u| u.login)
        );
        send_event(Event::PullRequestApproved(pr, review)).await;
    }
}

async fn handle_pr_event(event: WebhookEvent) {
    let WebhookEventPayload::PullRequest(event) = event.specific else {
        error!("Invalid pull request event payload!");
        return;
    };

    match event.action {
        PullRequestWebhookEventAction::Opened => handle_pr_created(event).await,
        PullRequestWebhookEventAction::ReadyForReview => handle_pr_ready(event).await,
        PullRequestWebhookEventAction::Closed => handle_pr_closed(event).await,
        PullRequestWebhookEventAction::ConvertedToDraft => handle_pr_drafted(event).await,
        PullRequestWebhookEventAction::Reopened => handle_pr_reopened(event).await,

        _ => trace!("Ignored PR webhook event action: {:?}", event.action),
    }
}

async fn handle_pr_review_event(event: WebhookEvent) {
    let WebhookEventPayload::PullRequestReview(event) = event.specific else {
        error!("Invalid pull request review event payload!");
        return;
    };

    match event.action {
        PullRequestReviewWebhookEventAction::Submitted => handle_pr_approved(event).await,
        _ => trace!("Ignored PR webhook event action: {:?}", event.action),
    }
}

async fn handle_pr_comment_event(event: WebhookEvent) {
    let WebhookEventPayload::PullRequestReviewComment(event) = event.specific else {
        error!("Invalid pull request review comment event payload!");
        return;
    };

    if event.action != PullRequestReviewCommentWebhookEventAction::Created {
        return;
    }

    send_event(Event::PullRequestComment(
        event.pull_request.number,
        event.comment.clone().body,
        event.comment.clone().user.unwrap().login,
    ))
    .await;
}

async fn handle_pr_thread_comment_event(event: WebhookEvent) {
    let WebhookEventPayload::PullRequestReviewThread(event) = event.specific else {
        error!("Invalid pull request review comment event payload!");
        return;
    };

    let Some(comment) = event.thread.comments.last() else {
        error!("Invalid thread comments!");
        return;
    };

    send_event(Event::PullRequestComment(
        event.pull_request.number,
        comment.clone().body,
        comment.clone().user.unwrap().login,
    ))
    .await;
}

// for some reason issue comments are the same as pr comments? this makes everything much more annoying
async fn handle_issue_comment(event: WebhookEvent) {
    let WebhookEventPayload::IssueComment(event) = event.specific else {
        error!("Invalid issue comment event payload!");
        return;
    };

    let Some(pr) = event.issue.pull_request else {
        return;
    };

    let path_segments = pr.html_url.path_segments().unwrap().collect::<Vec<&str>>();
    let Ok(id) = path_segments.last().unwrap().parse::<u64>() else {
        error!("Invalid PR URL: {}", pr.html_url);
        return;
    };

    send_event(Event::PullRequestComment(
        id,
        event.comment.clone().body.unwrap(),
        event.comment.clone().user.login,
    ))
    .await;
}

async fn push(
    Path(actual_secret): Path<String>,
    State(expected_secret): State<Arc<Secret>>,
    request: Request<Body>,
) {
    if actual_secret != expected_secret.0.clone() {
        trace!("Attempted push with incorrect secret");
        return;
    }

    let (parts, body) = request.into_parts();
    let Some(header) = parts.headers.get("X-GitHub-Event") else {
        error!("Malformed webhook event");
        return;
    };
    let body = axum::body::to_bytes(body, usize::MAX)
        .await
        .expect("failed converting body to bytes");

    let event = WebhookEvent::try_from_header_and_body(header.to_str().unwrap(), &body).unwrap();
    match event.kind {
        WebhookEventType::PullRequest => handle_pr_event(event).await,
        WebhookEventType::PullRequestReview => handle_pr_review_event(event).await,
        WebhookEventType::PullRequestReviewComment => handle_pr_comment_event(event).await,
        WebhookEventType::PullRequestReviewThread => handle_pr_thread_comment_event(event).await,
        WebhookEventType::IssueComment => handle_issue_comment(event).await,

        _ => trace!("Webhook event of kind {:?}", event.kind),
    }
}

pub async fn setup_webhook() {
    let Ok(secret) = env::var("WEBHOOK_SECRET") else {
        error!("Missing env var WEBHOOK_SECRET - webhook will not work.");
        return;
    };

    let app = Router::new()
        .route("/push/{actual_secret}", post(push))
        .with_state(Arc::new(Secret(secret)));
    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap()
}
