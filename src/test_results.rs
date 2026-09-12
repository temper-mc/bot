use poise::serenity_prelude::{CreateEmbed, CreateEmbedFooter, CreateMessage};

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
        .description(format!("Results for commit `{}`.", report.commit))
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
    let lines = body
        .lines()
        .map(clean_line)
        .filter(|line| !line.is_empty() && !line.starts_with("[test-results]:"))
        .collect::<Vec<_>>();

    if lines.first()? != "Test Results" {
        return None;
    }

    let summary = lines.get(1)?;
    let tests = lines.get(2)?;
    let runs = lines.get(3)?;
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

fn counts_for(parts: &[&str], label: &str) -> Option<ResultCounts> {
    let label_index = parts.iter().position(|part| *part == label)?;
    let values = parts.get(label_index + 1..)?;

    if values.len() != 3 {
        return None;
    }

    Some(ResultCounts {
        total: parse_count(parts.get(..label_index)?.join(""))?,
        passed: parse_count(values[0])?,
        skipped: parse_count(values[1])?,
        failed: parse_count(values[2])?,
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
                commit: "511c68ae".to_string(),
            }
        );
    }

    #[test]
    fn ignores_regular_comments() {
        assert_eq!(parse_test_results("just a normal comment"), None);
    }
}
