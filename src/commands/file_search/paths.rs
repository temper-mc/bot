use poise::command;
use poise::serenity_prelude::Error as SerenityError;
use tracing::error;
use crate::CmdContext;
use crate::commands::file_search;
use crate::commands::file_search::{fuzzy, git, setup_repo, REPO_PATH};

#[command(
    slash_command,
    prefix_command,
    description_localized("en-US", "Search for files in the repository"),
    aliases("f", "file", "fuzzy")
)]
pub async fn file_search(
    ctx: CmdContext<'_>,
    #[description = "Search query"] query: String,
    #[description = "How many results to return (default: 3)"]
    #[min = 1]
    #[max = 20]
    limit: Option<u8>,
) -> Result<(), SerenityError> {

    let res_count = limit.unwrap_or(3);
    setup_repo(&ctx).await?;

    let results = fuzzy::fuzzy_search_dir(&query, REPO_PATH.into());

    if results.is_empty() {
        ctx.say(format!("No files found for query `{}`", query))
            .await?;
    } else {
        let response = results
            .into_iter()
            .take(res_count as usize)
            .map(|p| format!("- {}", file_search::to_link(p.to_string_lossy().to_string(), None)))
            .collect::<Vec<_>>()
            .join("\n");
        ctx.say(format!("Found files for query `{query}`:\n{response}"))
            .await?;
    }

    Ok(())
}