use git2::{
    build::CheckoutBuilder, AnnotatedCommit, Error, FetchOptions, MergeAnalysis,
    RemoteCallbacks, Repository,
};

pub fn git_clone(repo_url: &str, repo_path: &str) -> Result<(), Error> {
    Repository::clone(repo_url, repo_path)?;
    Ok(())
}

pub fn git_pull(repo_path: &str) -> Result<(), Error> {
    let repo = Repository::open(repo_path)?;

    // Get current branch
    let head = repo.head()?;
    let branch = head.shorthand().ok_or_else(|| Error::from_str("Invalid branch name"))?;
    let branch_ref = format!("refs/heads/{}", branch);

    // Setup authentication callbacks (works for public repos and most ssh setups)
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username, _allowed| {
        git2::Cred::ssh_key_from_agent(username.unwrap())
    });

    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);

    // Fetch origin
    let mut remote = repo.find_remote("origin")?;
    remote.fetch(&[branch], Some(&mut fetch_options), None)?;

    // Get FETCH_HEAD
    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;

    // Analyse merge
    let (analysis, _) = repo.merge_analysis(&[&fetch_commit])?;

    if analysis.contains(MergeAnalysis::ANALYSIS_FASTFORWARD) {
        fast_forward(&repo, &branch_ref, &fetch_commit)?;
    } else if analysis.contains(MergeAnalysis::ANALYSIS_UP_TO_DATE) {
        // Nothing to do
    } else {
        return Err(Error::from_str("Non fast-forward merge not supported"));
    }

    Ok(())
}

fn fast_forward(
    repo: &Repository,
    local_branch: &str,
    fetch_commit: &AnnotatedCommit,
) -> Result<(), Error> {

    let mut reference = repo.find_reference(local_branch)?;

    reference.set_target(fetch_commit.id(), "Fast-Forward")?;
    repo.set_head(local_branch)?;

    repo.checkout_head(Some(
        CheckoutBuilder::default()
            .allow_conflicts(false)
            .force(),
    ))?;

    Ok(())
}
