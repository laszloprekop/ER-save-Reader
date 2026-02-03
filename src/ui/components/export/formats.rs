//! Export format implementations.

use serde::Serialize;

/// Export format options
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExportFormat {
    #[default]
    Json,
    Csv,
    Markdown,
}

impl ExportFormat {
    pub fn label(&self) -> &'static str {
        match self {
            ExportFormat::Json => "JSON",
            ExportFormat::Csv => "CSV",
            ExportFormat::Markdown => "Markdown",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Json => "json",
            ExportFormat::Csv => "csv",
            ExportFormat::Markdown => "md",
        }
    }
}

/// Format data as JSON
pub fn to_json<T: Serialize>(data: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(data)
}

/// Format tabular data as CSV
pub fn to_csv(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut output = String::new();

    // Header row
    output.push_str(&headers.join(","));
    output.push('\n');

    // Data rows
    for row in rows {
        let escaped: Vec<String> = row.iter()
            .map(|cell| {
                if cell.contains(',') || cell.contains('"') || cell.contains('\n') {
                    format!("\"{}\"", cell.replace('"', "\"\""))
                } else {
                    cell.clone()
                }
            })
            .collect();
        output.push_str(&escaped.join(","));
        output.push('\n');
    }

    output
}

/// Format tabular data as Markdown table
pub fn to_markdown(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut output = String::new();

    // Header row
    output.push_str("| ");
    output.push_str(&headers.join(" | "));
    output.push_str(" |\n");

    // Separator row
    output.push_str("| ");
    output.push_str(&headers.iter().map(|_| "---").collect::<Vec<_>>().join(" | "));
    output.push_str(" |\n");

    // Data rows
    for row in rows {
        output.push_str("| ");
        output.push_str(&row.join(" | "));
        output.push_str(" |\n");
    }

    output
}

/// Format data as tab-separated for clipboard
pub fn to_tsv(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut output = String::new();

    // Header row
    output.push_str(&headers.join("\t"));
    output.push('\n');

    // Data rows
    for row in rows {
        output.push_str(&row.join("\t"));
        output.push('\n');
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_simple() {
        let headers = &["Name", "Value"];
        let rows = vec![
            vec!["Item 1".to_string(), "100".to_string()],
            vec!["Item 2".to_string(), "200".to_string()],
        ];

        let csv = to_csv(headers, &rows);
        assert!(csv.contains("Name,Value"));
        assert!(csv.contains("Item 1,100"));
    }

    #[test]
    fn test_csv_escaping() {
        let headers = &["Name"];
        let rows = vec![
            vec!["Item with, comma".to_string()],
            vec!["Item with \"quotes\"".to_string()],
        ];

        let csv = to_csv(headers, &rows);
        assert!(csv.contains("\"Item with, comma\""));
        assert!(csv.contains("\"Item with \"\"quotes\"\"\""));
    }

    #[test]
    fn test_markdown() {
        let headers = &["Name", "Value"];
        let rows = vec![
            vec!["Item 1".to_string(), "100".to_string()],
        ];

        let md = to_markdown(headers, &rows);
        assert!(md.contains("| Name | Value |"));
        assert!(md.contains("| --- | --- |"));
        assert!(md.contains("| Item 1 | 100 |"));
    }
}
