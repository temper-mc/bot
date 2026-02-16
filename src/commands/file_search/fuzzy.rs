use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use std::path::PathBuf;

const EXCLUDED_DIRS: [&str; 3] = ["target", ".git", ".etc"];

pub fn fuzzy_search_dir(query: &str, dir: PathBuf) -> Vec<PathBuf> {
    let mut entries = vec![];
    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| !EXCLUDED_DIRS.contains(&e.file_name().to_str().unwrap_or("")))
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            entries.push(entry.path().to_string_lossy().to_string());
        }
    }

    let mut matcher = nucleo_matcher::Matcher::default();

    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);

    pattern
        .match_list(entries, &mut matcher)
        .into_iter()
        .map(|m| {
            let path = m.0.replace("\\", "/");
            let stripped = path.strip_prefix("./repo/").unwrap_or(&path);
            PathBuf::from(stripped)
        })
        .collect()
}
