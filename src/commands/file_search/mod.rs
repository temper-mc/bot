use crate::CmdContext;
use poise::command;
use poise::serenity_prelude::prelude::SerenityError;
use tracing::error;

mod fuzzy;
mod git;
pub mod text;
mod rg;

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
    const REPO_PATH: &str = "./repo";

    let res_count = limit.unwrap_or(3);
    if !std::path::Path::new(REPO_PATH).exists() {
        ctx.reply("Git repo needs to be cloned, this may take a moment...")
            .await?;
        ctx.defer().await?;
        git::git_clone("https://github.com/temper-mc/temper.git", REPO_PATH).map_err(|err| {
            error!("Failed to clone repository: {err}");
            SerenityError::Other("Failed to clone repository")
        })?;
    } else {
        ctx.defer().await?;
        git::git_pull(REPO_PATH).map_err(|err| {
            error!("Failed to pull repository: {err}");
            SerenityError::Other("Failed to pull repository")
        })?;
    }

    let results = fuzzy::fuzzy_search_dir(&query, REPO_PATH.into());

    if results.is_empty() {
        ctx.say(format!("No files found for query `{}`", query))
            .await?;
    } else {
        let response = results
            .into_iter()
            .take(res_count as usize)
            .map(|p| format!("- {}", to_link(p.to_string_lossy().to_string(), None)))
            .collect::<Vec<_>>()
            .join("\n");
        ctx.say(format!("Found files for query `{query}`:\n{response}"))
            .await?;
    }

    Ok(())
}

fn to_link(path: String, line: Option<u64>) -> String {
    let line_suffix = line.map_or_else(String::new, |l| format!("#L{l}"));
    format!(
        "[{}](<https://github.com/temper-mc/temper/blob/master/{}{}>)",
        path,
        path.replace("\\", "/"),
        line_suffix
    )
}
