use std::collections::HashSet;

use serde::Serialize;

const LOG_TIME_FORMAT: &str = "%Y.%m.%d %H:%M:%S";
const LOG_LEVELS: [&str; 3] = ["Debug", "Warning", "Error"];

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub category: Option<String>,
    pub message: String,
    pub raw: String,
    pub line_number: usize,
    pub end_line_number: usize,
    pub file_name: String,
    pub continuation_lines: Vec<String>,
}

pub struct LogEntryFilter {
    query: Option<String>,
    levels: Option<HashSet<String>>,
    categories: Option<HashSet<String>>,
}

impl LogEntryFilter {
    pub fn from_parts(
        query: Option<String>,
        levels: Option<Vec<String>>,
        categories: Option<Vec<String>>,
    ) -> Self {
        let query = query
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty());
        let levels = normalize_level_set(levels);
        let categories = categories
            .unwrap_or_default()
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect::<HashSet<_>>();

        Self {
            query,
            levels,
            categories: (!categories.is_empty()).then_some(categories),
        }
    }

    pub fn matches(&self, entry: &LogEntry) -> bool {
        if let Some(levels) = &self.levels {
            if !levels.contains(&entry.level) {
                return false;
            }
        }

        if let Some(categories) = &self.categories {
            if !entry
                .category
                .as_deref()
                .is_some_and(|category| categories.contains(category))
            {
                return false;
            }
        }

        if let Some(query) = &self.query {
            let continuation_text = entry.continuation_lines.join("\n");
            let haystack = [
                entry.timestamp.as_str(),
                entry.level.as_str(),
                entry.category.as_deref().unwrap_or_default(),
                entry.message.as_str(),
                entry.raw.as_str(),
                continuation_text.as_str(),
            ]
            .join("\n")
            .to_ascii_lowercase();
            if !haystack.contains(query) {
                return false;
            }
        }

        true
    }
}

pub struct ParsedLogDocument {
    pub entries: Vec<LogEntry>,
    pub total_lines: usize,
}

pub fn parse_log_document(file_name: &str, content: &str) -> ParsedLogDocument {
    ParsedLogDocument {
        entries: parse_log_entries(file_name, content),
        total_lines: content.lines().count(),
    }
}

pub fn parse_log_entries(file_name: &str, content: &str) -> Vec<LogEntry> {
    let mut entries = Vec::new();
    let mut current: Option<LogEntry> = None;

    for (index, line) in content.lines().enumerate() {
        let line_number = index + 1;
        if let Some((timestamp, level, message)) = parse_log_header(line) {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            current = Some(LogEntry {
                timestamp,
                level,
                category: extract_category(&message),
                message,
                raw: line.to_string(),
                line_number,
                end_line_number: line_number,
                file_name: file_name.to_string(),
                continuation_lines: Vec::new(),
            });
            continue;
        }

        if let Some(entry) = &mut current {
            entry.continuation_lines.push(line.to_string());
            entry.end_line_number = line_number;
        }
    }

    if let Some(entry) = current {
        entries.push(entry);
    }
    entries
}

pub(crate) fn parse_log_header(line: &str) -> Option<(String, String, String)> {
    let timestamp = line.get(..19)?;
    chrono::NaiveDateTime::parse_from_str(timestamp, LOG_TIME_FORMAT).ok()?;
    let rest = line.get(19..)?.trim_start();

    for level in LOG_LEVELS {
        let Some(after_level) = rest.strip_prefix(level) else {
            continue;
        };
        let message = after_level
            .trim_start()
            .strip_prefix('-')?
            .trim_start()
            .to_string();
        return Some((timestamp.to_string(), level.to_string(), message));
    }

    None
}

pub(crate) fn extract_category(message: &str) -> Option<String> {
    let trimmed = message.trim_start();
    let category = trimmed
        .strip_prefix('[')?
        .split_once(']')?
        .0
        .trim()
        .to_string();
    (!category.is_empty()).then_some(category)
}

pub(crate) fn normalize_level_set(levels: Option<Vec<String>>) -> Option<HashSet<String>> {
    let normalized = levels
        .unwrap_or_default()
        .into_iter()
        .filter_map(|level| match level.trim().to_ascii_lowercase().as_str() {
            "debug" => Some("Debug".to_string()),
            "warning" | "warn" => Some("Warning".to_string()),
            "error" => Some("Error".to_string()),
            _ => None,
        })
        .collect::<HashSet<_>>();
    (!normalized.is_empty()).then_some(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entries_with_categories_and_continuations() {
        let content = "\
2026.06.21 12:00:01 Debug - [Behaviour] first line
continued detail
2026.06.21 12:00:02 Warning - no category
2026.06.21 12:00:03 Error - [Network] failed";

        let ParsedLogDocument {
            entries,
            total_lines,
        } = parse_log_document("output_log_2026-06-21.txt", content);

        assert_eq!(total_lines, 4);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].category.as_deref(), Some("Behaviour"));
        assert_eq!(entries[0].line_number, 1);
        assert_eq!(entries[0].end_line_number, 2);
        assert_eq!(entries[0].continuation_lines, vec!["continued detail"]);
        assert_eq!(entries[1].category, None);
        assert_eq!(entries[2].level, "Error");
    }

    #[test]
    fn ignores_lines_before_first_header() {
        let content = "\
orphan line
2026.06.21 12:00:01 Debug - [Behaviour] first line";

        let entries = parse_log_entries("output_log_2026-06-21.txt", content);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].line_number, 2);
        assert!(entries[0].continuation_lines.is_empty());
    }

    #[test]
    fn filter_normalizes_levels_and_searches_all_text() {
        let entries = parse_log_entries(
            "output_log_2026-06-21.txt",
            "\
2026.06.21 12:00:01 Debug - [Behaviour] first line
continued needle
2026.06.21 12:00:02 Warning - [Other] second line",
        );
        let filter = LogEntryFilter::from_parts(
            Some("NEEDLE".to_string()),
            Some(vec!["warn".to_string(), "DEBUG".to_string()]),
            Some(vec!["Behaviour".to_string()]),
        );

        assert!(filter.matches(&entries[0]));
        assert!(!filter.matches(&entries[1]));
    }

    #[test]
    fn rejects_malformed_headers() {
        assert!(parse_log_header("2026.06.21 12:00:01 Info - message").is_none());
        assert!(parse_log_header("2026.06.21 12:00:01 Debug message").is_none());
        assert!(parse_log_header("not a timestamp Debug - message").is_none());
    }
}
