use std::io::Read;

use base64::{Engine, engine::general_purpose::STANDARD};
use flate2::read::GzDecoder;
use poise::serenity_prelude::{CreateEmbed, CreateEmbedFooter, CreateMessage};
use serde::Deserialize;

const TEST_RESULTS_DATA_PREFIX: &str = "[test-results]:data:application/gzip;base64,";

#[derive(Debug, PartialEq, Eq)]
struct ResultCounts {
    total: u64,
    passed: u64,
    skipped: u64,
    failed: u64,
}

#[derive(Debug, PartialEq, Eq)]
struct TestResults {
    files: u64,
    suites: u64,
    duration: String,
    tests: ResultCounts,
    runs: ResultCounts,
    commit: String,
}

pub fn test_results_message(body: &str, user: &str) -> Option<CreateMessage> {
    let report = parse_test_results(body)?;
    let color = if report.tests.failed == 0 && report.runs.failed == 0 {
        0x2e7d32
    } else {
        0xc62828
    };

    let embed = CreateEmbed::new()
        .title("Test Results")
        .description(format!(
            "Results for commit `{}`.",
            display_commit(&report.commit)
        ))
        .color(color)
        .field(
            "Summary",
            format!(
                "`{}` files\n`{}` suites\n`{}`",
                format_count(report.files),
                format_count(report.suites),
                report.duration
            ),
            true,
        )
        .field("Tests", format_counts(&report.tests), true)
        .field("Runs", format_counts(&report.runs), true)
        .footer(CreateEmbedFooter::new(format!("Posted by {user}")));

    Some(CreateMessage::new().embed(embed))
}

fn parse_test_results(body: &str) -> Option<TestResults> {
    parse_embedded_test_results(body).or_else(|| parse_visible_test_results(body))
}

fn parse_embedded_test_results(body: &str) -> Option<TestResults> {
    let payload = body
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix(TEST_RESULTS_DATA_PREFIX))?;
    let compressed = STANDARD.decode(payload).ok()?;
    let mut decoder = GzDecoder::new(compressed.as_slice());
    let mut data = String::new();
    decoder.read_to_string(&mut data).ok()?;

    EmbeddedTestResults::from_json(&data)
}

fn parse_visible_test_results(body: &str) -> Option<TestResults> {
    let lines = body
        .lines()
        .map(clean_line)
        .filter(|line| !is_report_metadata(line))
        .collect::<Vec<_>>();

    let title_index = lines.iter().position(|line| line == "Test Results")?;
    let summary = lines.get(title_index + 1)?;
    let tests = lines.get(title_index + 2)?;
    let runs = lines.get(title_index + 3)?;
    let commit = lines
        .iter()
        .find_map(|line| line.strip_prefix("Results for commit "))
        .and_then(|commit| commit.strip_suffix('.'))?;

    let summary = summary.split_whitespace().collect::<Vec<_>>();
    let tests = tests.split_whitespace().collect::<Vec<_>>();
    let runs = runs.split_whitespace().collect::<Vec<_>>();

    let files = count_before(&summary, "files")?;
    let suites = count_before(&summary, "suites")?;
    let duration = summary
        .iter()
        .position(|part| *part == "suites")
        .and_then(|index| {
            let duration = summary.get(index + 1..)?.join(" ");
            (!duration.is_empty()).then_some(duration)
        })?;

    Some(TestResults {
        files,
        suites,
        duration,
        tests: counts_for(&tests, "tests")?,
        runs: counts_for(&runs, "runs")?,
        commit: commit.to_string(),
    })
}

#[derive(Deserialize)]
struct EmbeddedTestResults {
    files: u64,
    suites: u64,
    duration: u64,
    tests: u64,
    tests_succ: u64,
    tests_skip: u64,
    tests_fail: u64,
    tests_error: u64,
    runs: u64,
    runs_succ: u64,
    runs_skip: u64,
    runs_fail: u64,
    runs_error: u64,
    commit: String,
}

impl EmbeddedTestResults {
    fn from_json(data: &str) -> Option<TestResults> {
        let report = serde_json::from_str::<Self>(data).ok()?;

        Some(TestResults {
            files: report.files,
            suites: report.suites,
            duration: format_duration(report.duration),
            tests: ResultCounts {
                total: report.tests,
                passed: report.tests_succ,
                skipped: report.tests_skip,
                failed: report.tests_fail + report.tests_error,
            },
            runs: ResultCounts {
                total: report.runs,
                passed: report.runs_succ,
                skipped: report.runs_skip,
                failed: report.runs_fail + report.runs_error,
            },
            commit: report.commit,
        })
    }
}

fn is_report_metadata(line: &str) -> bool {
    line.is_empty()
        || line.starts_with(TEST_RESULTS_DATA_PREFIX)
        || line.contains("This comment has been updated with latest results.")
}

fn counts_for(parts: &[&str], label: &str) -> Option<ResultCounts> {
    let label_index = parts.iter().position(|part| *part == label)?;
    let values = parts
        .get(label_index + 1..)?
        .iter()
        .filter_map(parse_count)
        .collect::<Vec<_>>();

    if values.len() < 3 {
        return None;
    }

    Some(ResultCounts {
        total: parse_count(parts.get(..label_index)?.join(""))?,
        passed: values[0],
        skipped: values[1],
        failed: values[2],
    })
}

fn count_before(parts: &[&str], label: &str) -> Option<u64> {
    let label_index = parts.iter().position(|part| *part == label)?;
    parse_count(parts.get(..label_index)?.last()?)
}

fn parse_count(value: impl AsRef<str>) -> Option<u64> {
    value.as_ref().parse().ok()
}

fn format_counts(counts: &ResultCounts) -> String {
    format!(
        "Total: `{}`\nPassed: `{}`\nSkipped: `{}`\nFailed: `{}`",
        format_count(counts.total),
        format_count(counts.passed),
        format_count(counts.skipped),
        format_count(counts.failed)
    )
}

fn format_count(count: u64) -> String {
    let count = count.to_string();
    let mut formatted = String::new();

    for (index, ch) in count.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            formatted.push(',');
        }

        formatted.push(ch);
    }

    formatted.chars().rev().collect()
}

fn format_duration(duration: u64) -> String {
    let minutes = duration / 60;
    let seconds = duration % 60;

    if minutes == 0 {
        format!("{seconds}s")
    } else {
        format!("{minutes}m {seconds}s")
    }
}

fn display_commit(commit: &str) -> &str {
    commit.get(..8).unwrap_or(commit)
}

fn clean_line(line: &str) -> String {
    let chars = line.chars().collect::<Vec<_>>();
    let mut cleaned = String::with_capacity(line.len());

    for (index, ch) in chars.iter().copied().enumerate() {
        if !is_spacing(ch) {
            cleaned.push(ch);
            continue;
        }

        let joins_digits = index
            .checked_sub(1)
            .and_then(|prev| chars.get(prev))
            .is_some_and(|ch| ch.is_ascii_digit())
            && chars.get(index + 1).is_some_and(|ch| ch.is_ascii_digit());

        if joins_digits {
            continue;
        }

        if !cleaned.ends_with(' ') {
            cleaned.push(' ');
        }
    }

    cleaned.trim().to_string()
}

fn is_spacing(ch: char) -> bool {
    ch.is_whitespace()
        || matches!(
            ch,
            '\u{2000}'..='\u{200b}' | '\u{202f}' | '\u{205f}' | '\u{3000}'
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPORT: &str = r#"Test Results
    4 files    138 suites   2m 25s 
  343 tests   342  0  1 
1 224 runs  1 223  0  1 

Results for commit 511c68ae.

[test-results]:data:application/gzip;base64,H4sIAIy3mmoC/02NQQ6DIBBFr2JYdyEMCPYyDQ6YkKo0CKumdy8QRZfv/cmbL5ndYnfy7PijI3tysQIFldGkoKPzWxFcZJHHWGbgcNJrT4hVsUu93ServolZu6VEmrAh+HCchLTVl4zxg85mVtDU1ax8S1a+F9Gvq4sZiKAUB6WtRE6NwFGbmYGZQErAfpiMVaBGaSn5/QHgu00uCQEAAA=="#;

    const DECORATED_REPORT: &str = r#"Test Results
    4 files    156 suites   6m 58s ⏱️
  347 tests   347 ✅ 0 💤 0 ❌
1 388 runs  1 388 ✅ 0 💤 0 ❌

Results for commit cb00d16d.

[test-results]:data:application/gzip;base64,H4sIALPVpGoC/12NSwqDMAAFryJZd5F/Yi9T8oVQNSWflfTujUFb7XLmwbwV+DC5DO4DvQ0g11A6IMYb2ppUCXHZViSbaGPZZkLFQY9cjflXz/BqCn6FV2G6CJdSTLtJdemXRMqdjuZF/ZqdT8nO56KJ8xxKA2A0hBZxiwTjjmhhsEFMWTRq7RRXwo+CYo05eH8A/d7c5AkBAAA=

:recycle: This comment has been updated with latest results."#;

    #[test]
    fn parses_publish_unit_test_result_comment() {
        let report = parse_test_results(REPORT).unwrap();

        assert_eq!(
            report,
            TestResults {
                files: 4,
                suites: 138,
                duration: "2m 25s".to_string(),
                tests: ResultCounts {
                    total: 343,
                    passed: 342,
                    skipped: 0,
                    failed: 1,
                },
                runs: ResultCounts {
                    total: 1224,
                    passed: 1223,
                    skipped: 0,
                    failed: 1,
                },
                commit: "511c68ae7c41d5c9adf23db3773c06bde83897e1".to_string(),
            }
        );
    }

    #[test]
    fn parses_updated_comment_notice() {
        let report = parse_test_results(&format!(
            ":recycle: This comment has been updated with latest results.\n\n{REPORT}\n\n:recycle: This comment has been updated with latest results."
        ))
        .unwrap();

        assert_eq!(report.commit, "511c68ae7c41d5c9adf23db3773c06bde83897e1");
        assert_eq!(report.tests.failed, 1);
        assert_eq!(report.runs.total, 1224);
    }

    #[test]
    fn parses_decorated_status_counts() {
        let report = parse_test_results(DECORATED_REPORT).unwrap();

        assert_eq!(report.commit, "cb00d16d1756e3b7c2c15ad19bbea6a7f9742b26");
        assert_eq!(report.duration, "6m 58s");
        assert_eq!(report.tests.total, 347);
        assert_eq!(report.tests.passed, 347);
        assert_eq!(report.tests.skipped, 0);
        assert_eq!(report.tests.failed, 0);
        assert_eq!(report.runs.total, 1388);
        assert_eq!(report.runs.passed, 1388);
        assert_eq!(report.runs.skipped, 0);
        assert_eq!(report.runs.failed, 0);
    }

    #[test]
    fn falls_back_to_visible_report() {
        let report = parse_test_results(
            r#"Test Results
    4 files    156 suites   6m 58s ⏱️
  347 tests   347 ✅ 0 💤 0 ❌
1 388 runs  1 388 ✅ 0 💤 0 ❌

Results for commit cb00d16d.

:recycle: This comment has been updated with latest results."#,
        )
        .unwrap();

        assert_eq!(report.commit, "cb00d16d");
        assert_eq!(report.duration, "6m 58s ⏱️");
        assert_eq!(report.runs.total, 1388);
    }

    #[test]
    fn ignores_regular_comments() {
        assert_eq!(parse_test_results("just a normal comment"), None);
    }
}
