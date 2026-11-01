use std::io::Write;

use crate::diagnostic::Diagnostic;
use crate::formatter::Formatter;
use crate::formatter::color::Color;

pub struct TextFormatter {
    pub color: Color,
    /// `-a`: remaining correctable offenses need `rubocop -A`.
    pub safe_autocorrect: bool,
}

impl Formatter for TextFormatter {
    fn format_to(
        &self,
        diagnostics: &[Diagnostic],
        files: &[std::path::PathBuf],
        out: &mut dyn Write,
    ) {
        for d in diagnostics {
            let _ = writeln!(out, "{}", d.render(self.color));
        }
        write_summary(
            self.color,
            diagnostics,
            files.len(),
            self.safe_autocorrect,
            out,
        );
    }
}

fn noun(n: usize, one: &'static str, many: &'static str) -> &'static str {
    if n == 1 { one } else { many }
}

fn count_fix_states(diagnostics: &[Diagnostic]) -> (usize, usize) {
    (
        diagnostics.iter().filter(|d| d.corrected).count(),
        diagnostics
            .iter()
            .filter(|d| d.correctable && !d.corrected)
            .count(),
    )
}

fn colored_offense_count(color: Color, n: usize) -> String {
    let text = format!("{n} {}", noun(n, "offense", "offenses"));
    if n == 0 {
        color.green(&text)
    } else {
        color.red(&text)
    }
}

fn correctable_clause(color: Color, n: usize, safe: bool) -> String {
    let (word, tail) = if safe {
        (
            noun(n, "more offense", "more offenses"),
            " can be corrected with `rubocop -A`",
        )
    } else {
        (noun(n, "offense", "offenses"), " autocorrectable")
    };
    format!("{}{tail}", color.yellow(&format!("{n} {word}")))
}

fn write_with_corrected(
    color: Color,
    files: &str,
    offenses: &str,
    n: usize,
    corrected: usize,
    correctable: usize,
    safe: bool,
    out: &mut dyn Write,
) {
    let corr_text = format!("{corrected} {}", noun(corrected, "offense", "offenses"));
    let corr = if corrected == n {
        color.green(&corr_text)
    } else {
        color.cyan(&corr_text)
    };
    if correctable == 0 {
        let _ = writeln!(
            out,
            "\n{files} inspected, {offenses} detected, {corr} corrected"
        );
        return;
    }
    let more = correctable_clause(color, correctable, safe);
    let _ = writeln!(
        out,
        "\n{files} inspected, {offenses} detected, {corr} corrected, {more}"
    );
}

pub(crate) fn write_summary(
    color: Color,
    diagnostics: &[Diagnostic],
    file_count: usize,
    safe: bool,
    out: &mut dyn Write,
) {
    let n = diagnostics.len();
    let offenses = colored_offense_count(color, n);
    let files = format!("{file_count} {}", noun(file_count, "file", "files"));
    let (corrected, correctable) = count_fix_states(diagnostics);
    if corrected == 0 && correctable == 0 {
        if n == 0 {
            let clean = color.green("no offenses");
            let _ = writeln!(out, "\n{files} inspected, {clean} detected");
        } else {
            let _ = writeln!(out, "\n{files} inspected, {offenses} detected");
        }
    } else if corrected > 0 {
        write_with_corrected(
            color,
            &files,
            &offenses,
            n,
            corrected,
            correctable,
            safe,
            out,
        );
    } else {
        let more = correctable_clause(color, correctable, safe);
        let _ = writeln!(out, "\n{files} inspected, {offenses} detected, {more}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{Location, Severity};

    fn diag(correctable: bool, corrected: bool) -> Diagnostic {
        Diagnostic {
            path: "a.rb".into(),
            location: Location { line: 1, column: 0 },
            severity: Severity::Convention,
            cop_name: "Style/ModuleFunction".into(),
            message: "msg".into(),
            corrected,
            correctable,
            source_line: Default::default(),
            highlight_length: 1,
        }
    }

    fn summary(diags: &[Diagnostic], safe: bool) -> String {
        let mut buf = Vec::new();
        write_summary(Color::resolve(Some(false)), diags, 1, safe, &mut buf);
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn plain_run_says_autocorrectable() {
        let s = summary(&[diag(true, false)], false);
        assert!(
            s.contains("1 file inspected, 1 offense detected, 1 offense autocorrectable"),
            "{s}"
        );
    }

    #[test]
    fn safe_run_points_at_unsafe_flag() {
        let s = summary(&[diag(true, false)], true);
        assert!(
            s.contains(
                "1 file inspected, 1 offense detected, 1 more offense can be corrected with `rubocop -A`"
            ),
            "{s}"
        );
    }

    #[test]
    fn safe_run_plural_after_partial_correction() {
        let diags = [diag(true, true), diag(true, false), diag(true, false)];
        let s = summary(&diags, true);
        assert!(
            s.contains("1 offense corrected, 2 more offenses can be corrected with `rubocop -A`"),
            "{s}"
        );
    }
}
