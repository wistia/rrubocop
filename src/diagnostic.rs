//! Offense diagnostics matching RuboCop text/JSON shapes.

use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Convention,
    Warning,
    Error,
    Fatal,
}

impl Severity {
    pub fn letter(&self) -> char {
        match self {
            Severity::Convention => 'C',
            Severity::Warning => 'W',
            Severity::Error => 'E',
            Severity::Fatal => 'F',
        }
    }

    pub fn from_str(s: &str) -> Option<Severity> {
        match s.to_lowercase().as_str() {
            "convention" | "c" => Some(Severity::Convention),
            "warning" | "w" => Some(Severity::Warning),
            "error" | "e" => Some(Severity::Error),
            "fatal" | "f" => Some(Severity::Fatal),
            _ => None,
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.letter())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    /// 1-indexed line number
    pub line: usize,
    /// 0-indexed column (character offset within the line)
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub path: String,
    pub location: Location,
    pub severity: Severity,
    pub cop_name: String,
    /// Annotated message (may include `Cop/Name: ` prefix per DisplayCopNames).
    pub message: String,
    #[serde(default)]
    pub corrected: bool,
    /// Cop supports autocorrect for this offense (RuboCop `[Correctable]`).
    #[serde(default)]
    pub correctable: bool,
    /// Source line text for clang-style output (no trailing newline).
    #[serde(
        default,
        with = "source_line",
        skip_serializing_if = "source_line::is_empty"
    )]
    pub source_line: std::sync::Arc<str>,
    /// Highlight width in display columns (default 1 → single `^`).
    #[serde(default = "default_highlight_len")]
    pub highlight_length: usize,
}

fn default_highlight_len() -> usize {
    1
}

impl Diagnostic {
    pub fn sort_key(&self) -> (&str, usize, usize) {
        (&self.path, self.location.line, self.location.column)
    }

    /// Clang/progress offense block (header + source + caret), RuboCop-compatible.
    pub fn render(&self, color: crate::formatter::color::Color) -> String {
        let path = smart_path(&self.path);
        let mut status = String::new();
        if self.corrected {
            status.push_str(&color.green("[Corrected] "));
        } else if self.correctable {
            status.push_str(&color.yellow("[Correctable] "));
        }
        let header = format!(
            "{}:{}:{}: {}: {}{}",
            color.cyan(&path),
            self.location.line,
            self.location.column + 1,
            color.severity_letter(self.severity),
            status,
            annotate_backticks(&self.message, color),
        );
        if self.source_line.is_empty() {
            return header;
        }
        let caret = clang_caret(
            &self.source_line,
            self.location.column,
            self.highlight_length,
        );
        format!("{header}\n{}\n{caret}", self.source_line)
    }
}

/// RuboCop PathUtil.smart_path: prefer cwd-relative, drop a leading `./`.
pub fn smart_path(path: &str) -> String {
    let path = path.strip_prefix("./").unwrap_or(path);
    let p = std::path::Path::new(path);
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(rel) = p.strip_prefix(&cwd) {
            let s = rel.to_string_lossy();
            if !s.is_empty() {
                return s.into_owned();
            }
        }
    }
    path.to_string()
}

/// RuboCop SimpleTextFormatter#annotate_message: strip `` `...` ``; yellow insides when colored.
fn annotate_backticks(msg: &str, color: crate::formatter::color::Color) -> String {
    let mut out = String::with_capacity(msg.len());
    let mut rest = msg;
    while let Some(start) = rest.find('`') {
        out.push_str(&rest[..start]);
        rest = &rest[start + 1..];
        match rest.find('`') {
            Some(end) => {
                out.push_str(&color.yellow(&rest[..end]));
                rest = &rest[end + 1..];
            }
            None => {
                out.push('`');
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Spaces/tabs preserving caret underline for `column`..`column+len` (0-based).
fn clang_caret(source_line: &str, column: usize, highlight_length: usize) -> String {
    let mut prefix = String::new();
    for (i, ch) in source_line.chars().enumerate() {
        if i >= column {
            break;
        }
        prefix.push(if ch == '\t' { '\t' } else { ' ' });
    }
    while prefix.chars().count() < column {
        prefix.push(' ');
    }
    let len = highlight_length.max(1);
    format!("{prefix}{}", "^".repeat(len))
}

/// Annotate a raw cop message like RuboCop::Cop::MessageAnnotator.
pub fn annotate_offense_message(
    raw: &str,
    cop_name: &str,
    display_cop_names: bool,
    extra_details: bool,
    details: Option<&str>,
    display_style_guide: bool,
    style_guide_url: Option<&str>,
) -> String {
    let mut message = if display_cop_names {
        format!("{cop_name}: {raw}")
    } else {
        raw.to_string()
    };
    if extra_details {
        if let Some(d) = details.filter(|s| !s.is_empty()) {
            message.push(' ');
            message.push_str(d);
        }
    }
    if display_style_guide {
        if let Some(url) = style_guide_url.filter(|s| !s.is_empty()) {
            message = format!("{message} ({url})");
        }
    }
    message
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.render(crate::formatter::color::Color::resolve(Some(false)))
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smart_path_strips_dot_slash() {
        assert_eq!(smart_path("./lib/a.rb"), "lib/a.rb");
        assert_eq!(smart_path("lib/a.rb"), "lib/a.rb");
    }

    #[test]
    fn smart_path_relativizes_under_cwd() {
        let cwd = std::env::current_dir().unwrap();
        let abs = cwd.join("lib/a.rb");
        assert_eq!(smart_path(&abs.to_string_lossy()), "lib/a.rb");
        let sibling = format!("{}2/x.rb", cwd.display());
        assert_eq!(smart_path(&sibling), sibling);
    }

    #[test]
    fn display_strips_message_backticks() {
        let d = Diagnostic {
            path: "./lib/a.rb".into(),
            location: Location {
                line: 10,
                column: 2,
            },
            severity: Severity::Convention,
            cop_name: "Metrics/AbcSize".into(),
            message: "Metrics/AbcSize: Assignment Branch Condition size for `foo` is too high."
                .into(),
            corrected: false,
            correctable: false,
            source_line: Default::default(),
            highlight_length: 1,
        };
        let s = d.to_string();
        assert!(s.starts_with("lib/a.rb:10:3: C: Metrics/AbcSize: "));
        assert!(s.contains("for foo is too high"));
        assert!(!s.contains("`foo`"));
    }

    #[test]
    fn render_correctable_and_caret() {
        use crate::formatter::color::Color;
        let d = Diagnostic {
            path: "lib/a.rb".into(),
            location: Location { line: 1, column: 0 },
            severity: Severity::Convention,
            cop_name: "Style/FrozenStringLiteralComment".into(),
            message: "Style/FrozenStringLiteralComment: Missing magic comment # frozen_string_literal: true.".into(),
            corrected: false,
            correctable: true,
            source_line: "class Foo".into(),
            highlight_length: 1,
        };
        let s = d.render(Color::resolve(Some(false)));
        assert!(s.contains("[Correctable] "));
        assert!(s.contains("\nclass Foo\n"));
        assert!(s.ends_with("\n^") || s.contains("\n^\n") || s.lines().last() == Some("^"));
    }

    #[test]
    fn clang_caret_multibyte_line() {
        use crate::parse::source::byte_index_to_column;
        let line = "Rails.logger.debug { \"Начинаю\" } # rubocop:disable Rails/Output";
        let col = byte_index_to_column(line, line.find("Rails/Output").unwrap());
        let caret = clang_caret(line, col, "Rails/Output".len());
        assert_eq!(
            caret,
            format!("{}{}", " ".repeat(col), "^".repeat("Rails/Output".len()))
        );
    }

    #[test]
    fn annotate_respects_display_cop_names() {
        let with = annotate_offense_message("msg", "Style/Foo", true, false, None, false, None);
        assert_eq!(with, "Style/Foo: msg");
        let without = annotate_offense_message("msg", "Style/Foo", false, false, None, false, None);
        assert_eq!(without, "msg");
    }
}

mod source_line;
