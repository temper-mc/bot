use crate::CmdContext;
use crate::commands::file_search::{REPO_PATH, rg, setup_repo, to_link};
use poise::command;
use poise::serenity_prelude::prelude::SerenityError;
use std::path::Path;
use tracing::error;

#[command(
    slash_command,
    prefix_command,
    description_localized("en-US", "Search for text in the repository")
)]
pub async fn text_search(
    ctx: CmdContext<'_>,
    #[description = "Search query"] query: String,
    #[description = "How many results to return (default: 3)"]
    #[min = 1]
    #[max = 20]
    limit: Option<u8>,
) -> Result<(), SerenityError> {
    if !which::which("rg").is_ok() {
        ctx.reply(
            "Ripgrep (rg) is not installed or not in PATH. Please install it to use this command.",
        )
        .await?;
        return Ok(());
    }

    setup_repo(&ctx).await?;

    let matches =
        rg::ripgrep_matches_as_json_array(&query, Path::new(REPO_PATH)).map_err(|err| {
            error!("Failed to search repository: {err}");
            SerenityError::Other("Failed to search repository")
        })?;

    let message = if matches.is_empty() {
        format!("No matches found for query `{query}`")
    } else {
        let response = matches
            .into_iter()
            .take(limit.unwrap_or(3) as usize)
            .map(|m| {
                let path = m.path.replace("\\", "/");
                let stripped_path = path.strip_prefix("./repo/").unwrap_or(&path);
                format!(
                    "- {}: `{}`",
                    to_link(stripped_path.to_string(), Some(m.line_number)),
                    m.line
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!("Found matches for query `{query}`:\n{response}")
    };

    ctx.say(message).await?;

    Ok(())
}
