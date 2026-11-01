//! `# rubocop:disable` / `enable` directive parsing.

use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

static MARKER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)# *rubocop *: *").unwrap());

#[derive(Default)]
pub struct Directives {
    /// Cop name (or department or "all") → disabled line numbers (trailing).
    line_disables: HashMap<String, HashSet<usize>>,
    /// Cop name → inclusive ranges disabled by block directives.
    range_disables: HashMap<String, Vec<(usize, usize)>>,
    open_blocks: HashMap<String, usize>,
    comment_lines: Vec<usize>,
    comment_starts: Vec<usize>,
}

impl Directives {
    pub(crate) fn comment_lines(&self) -> &[usize] {
        &self.comment_lines
    }

    pub(crate) fn comment_line<'a>(
        &self,
        source: &'a crate::parse::source::SourceFile,
        line: usize,
    ) -> Option<std::borrow::Cow<'a, str>> {
        let index = self.comment_lines.binary_search(&line).ok()?;
        let text = source.line_text(line)?;
        let prefix = self.comment_starts[index] - source.line_start(line)?;
        Some(mask_prefix(text, prefix))
    }

    pub fn suppresses(&self, cop_name: &str, line: usize) -> bool {
        if self.line_disabled("all", line) || self.range_disabled("all", line) {
            return true;
        }
        if self.line_disabled(cop_name, line) || self.range_disabled(cop_name, line) {
            return true;
        }
        if let Some((dept, _)) = cop_name.split_once('/') {
            self.line_disabled(dept, line) || self.range_disabled(dept, line)
        } else {
            false
        }
    }

    fn line_disabled(&self, name: &str, line: usize) -> bool {
        self.line_disables
            .iter()
            .any(|(k, s)| k.eq_ignore_ascii_case(name) && s.contains(&line))
    }

    fn range_disabled(&self, name: &str, line: usize) -> bool {
        self.range_disables.iter().any(|(k, ranges)| {
            k.eq_ignore_ascii_case(name) && ranges.iter().any(|&(a, b)| a <= line && line <= b)
        })
    }
}

fn mask_prefix(text: &str, prefix: usize) -> std::borrow::Cow<'_, str> {
    if text.as_bytes()[..prefix]
        .iter()
        .all(u8::is_ascii_whitespace)
    {
        return std::borrow::Cow::Borrowed(text);
    }
    let mut masked = text.as_bytes().to_vec();
    for byte in &mut masked[..prefix] {
        if byte.is_ascii() && !byte.is_ascii_whitespace() {
            *byte = b'_';
        }
    }
    std::borrow::Cow::Owned(String::from_utf8(masked).unwrap())
}

fn apply_enable(d: &mut Directives, rest: &str, line_no: usize) {
    for name in cop_names(rest.trim_start_matches([' ', ':'])) {
        let Some(key) = d
            .open_blocks
            .keys()
            .find(|k| k.eq_ignore_ascii_case(&name))
            .cloned()
        else {
            continue;
        };
        if let Some(start) = d.open_blocks.remove(&key) {
            d.range_disables
                .entry(key)
                .or_default()
                .push((start, line_no));
        }
    }
}

fn apply_disable(d: &mut Directives, rest: &str, line: &str, line_no: usize) {
    let names = cop_names(rest.trim_start_matches([' ', ':']));
    // Bare `# rubocop:disable` is malformed (Lint/CopDirectiveSyntax), not "all".
    if names.is_empty() {
        return;
    }
    let trailing = !line.trim_start().starts_with('#');
    for name in names {
        if trailing {
            d.line_disables.entry(name).or_default().insert(line_no);
        } else {
            d.open_blocks.entry(name).or_insert(line_no);
        }
    }
}

fn apply_directive(d: &mut Directives, comment: &str, line: &str, line_no: usize) {
    if let Some(rest) = comment.strip_prefix("enable") {
        apply_enable(d, rest, line_no);
        return;
    }
    if let Some(rest) = comment
        .strip_prefix("disable")
        .or_else(|| comment.strip_prefix("todo"))
    {
        apply_disable(d, rest, line, line_no);
    }
}

mod source;
pub use source::parse;
pub(crate) use source::{cached, for_source};

fn cop_names(after: &str) -> Vec<String> {
    let after = after.split("--").next().unwrap_or(after);
    after
        .split(',')
        .map(|s| s.trim().trim_start_matches(':').replace("::", "/"))
        .map(qualify)
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_names_disable_the_qualified_cop() {
        let d =
            parse("# rubocop : disable LineLength\nwork\n# rubocop : enable LineLength\nmore\n");
        assert!(d.suppresses("Layout/LineLength", 2));
        assert!(!d.suppresses("Layout/LineLength", 4));
        let d = parse("work # rubocop:todo NoSetex, Lint\n");
        assert!(d.suppresses("Cops/NoSetex", 1));
        assert!(d.suppresses("Lint/UselessTimes", 1));
    }

    #[test]
    fn legacy_department_separator() {
        let d = parse(
            "# rubocop:disable Rails::SkipsModelValidations\nRecord.update_all(active: true)\n# rubocop:enable Rails::SkipsModelValidations\nRecord.update_all(active: false)\n",
        );
        assert!(d.suppresses("Rails/SkipsModelValidations", 2));
        assert!(!d.suppresses("Rails/SkipsModelValidations", 4));
    }

    #[test]
    fn chained_todo_comments() {
        let d = parse(
            "rescue Exception # rubocop:todo Lint/RescueException # rubocop:todo Cops/RescueMustReportError\n",
        );
        assert!(d.suppresses("Lint/RescueException", 1));
        assert!(d.suppresses("Cops/RescueMustReportError", 1));
    }

    #[test]
    fn disable_after_interpolation_hash() {
        let d = parse("x = \"a #{b}\" # rubocop:disable Style/StringLiterals\n");
        assert!(d.suppresses("Style/StringLiterals", 1));
    }

    #[test]
    fn disable_after_other_hash_in_comment() {
        let d = parse("x = 1 # priv: \"abc\" # rubocop:disable Lint/UnreachableCode\n");
        assert!(d.suppresses("Lint/UnreachableCode", 1));
    }

    #[test]
    fn disable_cop_name_is_case_insensitive() {
        let d = parse("it 'x' do end # rubocop:disable Rspec/ExampleLength\n");
        assert!(d.suppresses("RSpec/ExampleLength", 1));
    }

    #[test]
    fn disable_department_is_case_insensitive() {
        let d = parse("# rubocop:disable Rspec\nx = 1\n");
        assert!(d.suppresses("RSpec/LetSetup", 2));
    }

    #[test]
    fn disable_after_escaped_single_quote() {
        let d = parse("it 'doesn\\'t twice' do # rubocop:disable RSpec/MultipleExpectations\n");
        assert!(d.suppresses("RSpec/MultipleExpectations", 1));
    }
}

fn qualify(name: String) -> String {
    crate::config::cop_names::qualified(&name)
        .map(|n| n.into_owned())
        .unwrap_or(name)
}

#[cfg(test)]
mod syntax_tests;
