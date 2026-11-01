//! `# rubocop:disable` / `enable` directive parsing.

use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

static MARKER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)# *rubocop *: *").unwrap());

type CachedRanges = Mutex<HashMap<String, Arc<Vec<(usize, usize)>>>>;

#[derive(Default)]
pub struct Directives {
    switches: Vec<Switch>,
    ranges: CachedRanges,
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
        !self.switches.is_empty()
            && self
                .cop_ranges(cop_name)
                .iter()
                .any(|&(start, end)| start <= line && line <= end)
    }

    pub(crate) fn disabled_by(&self, cop_name: &str, line: usize, directive_line: usize) -> bool {
        !self.switches.is_empty()
            && self
                .cop_ranges(cop_name)
                .iter()
                .any(|&(start, end)| start == directive_line && start <= line && line <= end)
    }
}

mod state;
use state::Switch;

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

fn apply_switch(d: &mut Directives, rest: &str, line: &str, line_no: usize, disabled: bool) {
    let single_line = !line.trim_start().starts_with('#');
    for name in cop_names(rest.trim_start_matches([' ', ':'])) {
        d.switches.push(Switch {
            name,
            line: line_no,
            disabled,
            single_line,
        });
    }
}

fn apply_directive(d: &mut Directives, comment: &str, line: &str, line_no: usize) {
    if let Some(rest) = comment.strip_prefix("enable") {
        apply_switch(d, rest, line, line_no, false);
        return;
    }
    if let Some(rest) = comment
        .strip_prefix("disable")
        .or_else(|| comment.strip_prefix("todo"))
    {
        apply_switch(d, rest, line, line_no, true);
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
