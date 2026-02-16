use std::io;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use serde_json::{json, Value};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct RipgrepMatch {
    pub path: String,
    pub line_number: u64,
    pub line: String,
}

pub fn ripgrep_matches_as_json_array(pattern: &str, search_path: &Path) -> io::Result<Vec<RipgrepMatch>> {
    let mut child = Command::new("rg")
        .arg("--json")
        .arg(pattern)
        .arg(search_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "failed to capture rg stdout"))?;

    let reader = BufReader::new(stdout);

    let mut matches = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let value: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if value["type"] == "match" {
            let data = &value["data"];

            let path = data["path"]["text"].as_str().unwrap_or("").to_string();
            let line_number = data["line_number"].as_u64().unwrap_or(0);
            let line_text = data["lines"]["text"].as_str().unwrap_or("").to_string();

            matches.push(json!({
                "path": path,
                "line_number": line_number,
                "line": line_text.trim_end(),
            }));
        }
    }

    let status = child.wait()?;
    
    if status.code().unwrap_or(2) > 1 {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("rg failed: {status}"),
        ));
    }
    
    let matches: Vec<RipgrepMatch> = matches.into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();
    
    Ok(matches)
}