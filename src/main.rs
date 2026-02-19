use std::{
    env,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use bot::{ENV_VARS, Event, TX, pr_discussion, webhook::setup_webhook};
use poise::{
    Framework, FrameworkOptions, Prefix, PrefixFrameworkOptions,
    serenity_prelude::{
        ClientBuilder, Context, CreateMessage, Error, EventHandler, GatewayIntents, GuildId,
        Member, async_trait,
    },
};
use tokio::sync::{
    Mutex,
    mpsc::{Receiver, channel},
};
use tracing::{error, warn};
pub struct MainLoop {
    main_loop_running: AtomicBool,
}

impl Default for MainLoop {
    fn default() -> Self {
        Self {
            main_loop_running: AtomicBool::new(false),
        }
    }
}

async fn run_main_loop(ctx: &Arc<Context>, rx: &mut Receiver<Event>) {
    while let Some(event) = rx.recv().await {
        match event {
            Event::PullRequestOpened(pr) => pr_discussion::pr_created(ctx, pr).await,
            Event::PullRequestReady(pr) => {
                pr_discussion::apply_tag(ctx, pr.number, ENV_VARS.tag_review_needed).await;
                pr_discussion::send_message(
                    ctx,
                    pr.number,
                    CreateMessage::new()
                        .content(format!("Pull request #{} **ready for review**!", pr.number)),
                )
                .await
            }
            Event::PullRequestApproved(pr, review) => {
                pr_discussion::apply_tag(ctx, pr.number, ENV_VARS.tag_approved).await;
                let user = review
                    .user
                    .map(|u| u.login)
                    .unwrap_or("unknown".to_string());
                pr_discussion::send_message(
                    ctx,
                    pr.number,
                    CreateMessage::new().content(format!(
                        "Pull request #{} was approved by **{}**!",
                        pr.number, user
                    )),
                )
                .await
            }
            Event::PullRequestMerged(pr) => {
                pr_discussion::apply_tag(ctx, pr.number, ENV_VARS.tag_merged).await;
                let user = pr
                    .merged_by
                    .map(|u| u.login)
                    .unwrap_or("unknown".to_string());
                pr_discussion::send_message(
                    ctx,
                    pr.number,
                    CreateMessage::new().content(format!(
                        "Pull request #{} was merged by **{}** :tada:!",
                        pr.number, user
                    )),
                )
                .await;
            }
            Event::PullRequestDrafted(pr) => {
                pr_discussion::apply_tag(ctx, pr.number, ENV_VARS.tag_draft).await
            }
            Event::PullRequestClosed(pr) => {
                pr_discussion::apply_tag(ctx, pr.number, ENV_VARS.tag_closed).await;
                pr_discussion::send_message(
                    ctx,
                    pr.number,
                    CreateMessage::new()
                        .content(format!("Pull request #{} was closed!", pr.number)),
                )
                .await;
            }
            Event::PullRequestComment(pr, comment, user) => {
                let comment = comment
                    .lines()
                    .map(|l| format!("> {l}"))
                    .collect::<Vec<String>>()
                    .join("\n");
                pr_discussion::send_message(
                    ctx,
                    pr,
                    CreateMessage::new().content(format!("{comment}\n~ {user}")),
                )
                .await;
            }
        }
    }
}

#[async_trait]
impl EventHandler for MainLoop {
    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        if !self.main_loop_running.load(Ordering::Relaxed) {
            let ctx = Arc::new(ctx);

            let (tx, mut rx) = channel::<Event>(16);
            TX.set(Mutex::new(tx)).unwrap();

            tokio::spawn(async move {
                loop {
                    run_main_loop(&ctx, &mut rx).await;
                    tokio::time::sleep(Duration::from_millis(100)).await
                }
            });

            self.main_loop_running.swap(true, Ordering::Relaxed);
        }
    }

    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        if let Err(err) = new_member.add_role(ctx, ENV_VARS.member_role).await {
            error!("Failed assigning member role to new member: {err}");
        }
    }
}

async fn setup_bot() {
    let token = env::var("TOKEN").expect("missing TOKEN env var");
    let guild = env::var("GUILD")
        .expect("missing GUILD env var")
        .parse::<u64>()
        .expect("invalid GUILD env var");
    let intents = GatewayIntents::all();

    let framework = Framework::<(), Error>::builder()
        .options(FrameworkOptions {
            prefix_options: PrefixFrameworkOptions {
                prefix: Some(".".into()),
                additional_prefixes: vec![Prefix::Literal("!")],
                ..Default::default()
            },
            commands: vec![
                bot::commands::file_search::paths::file_search(),
                bot::commands::file_search::text::text_search(),
                bot::commands::merge::merge(),
            ],
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_in_guild(
                    ctx,
                    &framework.options().commands,
                    GuildId::new(guild),
                )
                .await?;
                Ok(())
            })
        })
        .build();

    let mut client = ClientBuilder::new(token, intents)
        .framework(framework)
        .event_handler(MainLoop::default())
        .await
        .unwrap();
    client.start().await.unwrap()
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    if let Err(err) = dotenvy::dotenv() {
        warn!("Failed loading .env: {err}");
    };

    tokio::spawn(async move { setup_webhook().await });

    setup_bot().await
}
