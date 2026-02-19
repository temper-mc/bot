use octocrab::{OctocrabBuilder, params::pulls::MergeMethod};
use poise::{CreateReply, serenity_prelude::Error};

use crate::{CmdContext, ENV_VARS, pr_discussion::find_pr_from_post};

#[poise::command(
    slash_command,
    prefix_command,
    hide_in_help,
    check = "check_maintainer"
)]
pub async fn merge(
    ctx: CmdContext<'_>,

    #[description = "Whether this should squash"]
    #[flag]
    squash: bool,

    #[description = "Whether this should rebase"]
    #[flag]
    rebase: bool,

    #[description = "Commit Message"]
    #[rest]
    msg: Option<String>,
) -> Result<(), Error> {
    let channel = ctx
        .guild_channel()
        .await
        .ok_or(Error::Other("Literally why are you doing this in DMs"))?;
    let id = find_pr_from_post(channel).ok_or(Error::Other("No PR provided!"))?;
    let method = if squash {
        MergeMethod::Squash
    } else if rebase {
        MergeMethod::Rebase
    } else {
        MergeMethod::Merge
    };

    // Allow having the message in an inline code block
    let msg = msg.map(|msg| {
        msg.strip_prefix('`')
            .unwrap_or(&msg)
            .strip_suffix('`')
            .unwrap_or(&msg)
            .to_string()
    });

    ctx.send(CreateReply::default().content("Attempting to merge..."))
        .await?;

    let client = OctocrabBuilder::new()
        .personal_token(ENV_VARS.github_token.clone())
        .build()
        .expect("failed building github client");

    let pr = client.pulls(ENV_VARS.repo_owner.to_string(), ENV_VARS.repo.to_string());
    let merge = pr
        .merge(id)
        .message(format!("Merged on Discord by {}", ctx.author().name))
        .method(method);

    // Jank because title() and the like consume self
    let merged = if let Some(msg) = msg {
        merge.title(msg).send().await
    } else {
        merge.send().await
    };

    if let Err(err) = merged {
        ctx.send(
            CreateReply::default().content(format!("Pull request #{id} failed to merge: {err}")),
        )
        .await?;
    };

    Ok(())
}

#[allow(dead_code)] // grrr
async fn check_maintainer(ctx: CmdContext<'_>) -> Result<bool, Error> {
    ctx.author()
        .has_role(ctx, ENV_VARS.guild, ENV_VARS.maintainer_role)
        .await
}
