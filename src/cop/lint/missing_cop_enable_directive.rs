use std::collections::HashMap;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// Lint/MissingCopEnableDirective — disable without matching enable at EOF.
pub struct MissingCopEnableDirective;

fn directive_names(rest: &str) -> Vec<String> {
    let names: Vec<String> = rest
        .trim()
        .trim_start_matches(':')
        .trim()
        .split("--")
        .next()
        .unwrap_or("")
        .split(',')
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
        .collect();
    if names.is_empty() {
        vec!["all".into()]
    } else {
        names
    }
}

fn apply_line(open: &mut HashMap<String, usize>, trimmed: &str, line: usize) {
    if let Some(rest) = trimmed.strip_prefix("# rubocop:disable") {
        for n in directive_names(rest) {
            open.entry(n).or_insert(line);
        }
    }
    if let Some(rest) = trimmed.strip_prefix("# rubocop:enable") {
        for n in directive_names(rest) {
            open.remove(&n);
        }
    }
}

fn report_open(
    cop: &MissingCopEnableDirective,
    source: &SourceFile,
    open: HashMap<String, usize>,
    last_line: usize,
    maximum: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (name, start) in open {
        if maximum != usize::MAX && last_line.saturating_sub(start) <= maximum {
            continue;
        }
        let directive = format!("# rubocop:enable {name}");
        diagnostics.push(cop.diagnostic(
            source,
            start,
            0,
            format!("Re-enable {name} cops with `{directive}` after disabling it."),
        ));
    }
}

impl Cop for MissingCopEnableDirective {
    fn name(&self) -> &'static str {
        "Lint/MissingCopEnableDirective"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn uses_source_phase(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _tree: &tree_sitter::Tree,
        _code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let maximum = config.get_usize("MaximumRangeSize", usize::MAX);
        if maximum == usize::MAX {
            return;
        }
        let mut open: HashMap<String, usize> = HashMap::new();
        let mut last_line = 0usize;
        for (i, line) in source.lines().enumerate() {
            last_line = i + 1;
            apply_line(&mut open, String::from_utf8_lossy(line).trim_start(), i + 1);
        }
        report_open(self, source, open, last_line, maximum, diagnostics);
    }
}
