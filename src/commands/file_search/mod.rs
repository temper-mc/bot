use poise::serenity_prelude::prelude::SerenityError;
use tracing::error;
use crate::CmdContext;

mod fuzzy;
mod git;
pub mod text;
mod rg;
pub mod paths;

const REPO_PATH: &str = "./repo";

fn to_link(path: String, line: Option<u64>) -> String {
    let line_suffix = line.map_or_else(String::new, |l| format!("#L{l}"));
    format!(
        "[{}](<https://github.com/temper-mc/temper/blob/master/{}{}>)",
        path,
        path.replace("\\", "/"),
        line_suffix
    )
}

async fn setup_repo(ctx: &CmdContext<'_>) -> Result<(), SerenityError> {

    if !std::path::Path::new(REPO_PATH).exists() {
        ctx.reply("Git repo needs to be cloned, this may take a moment...")
            .await?;
        ctx.defer().await?;
        tokio::task::spawn_blocking(|| {
            git::git_clone("https://github.com/temper-mc/temper.git", REPO_PATH).map_err(|err| {
                error!("Failed to clone repository: {err}");
                SerenityError::Other("Failed to clone repository")
            })
        }).await.map_err(|err| {
            error!("Failed to clone repository: {err}");
            SerenityError::Other("Failed to clone repository")
        })??;
    } else {
        ctx.defer().await?;
        git::git_pull(REPO_PATH).map_err(|err| {
            error!("Failed to pull repository: {err}");
            SerenityError::Other("Failed to pull repository")
        })?;
    };
    Ok(())
}
