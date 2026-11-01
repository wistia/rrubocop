use std::collections::HashSet;

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

use super::redundant_cop_disable_directive::nth_cop_token;

/// Lint/RedundantCopEnableDirective — enable without matching disable.
pub struct RedundantCopEnableDirective;
mod state;
use state::Disabled;

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

fn config_disabled(config: &CopConfig) -> HashSet<String> {
    match config.options.get("ConfigDisabledCops") {
        Some(serde_yml::Value::Sequence(items)) => items
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        _ => HashSet::new(),
    }
}

fn ws_comma_right(line: &str, mut end: usize) -> usize {
    while end < line.len() && line.as_bytes()[end].is_ascii_whitespace() {
        end += 1;
    }
    if end < line.len() && line.as_bytes()[end] == b',' {
        end += 1;
    }
    while end < line.len() && line.as_bytes()[end].is_ascii_whitespace() {
        end += 1;
    }
    end
}

fn partial_token_span(line: &str, byte: usize, len: usize) -> (usize, usize) {
    let mut left = byte;
    while left > 0 && line.as_bytes()[left - 1].is_ascii_whitespace() {
        left -= 1;
    }
    if left > 0 && line.as_bytes()[left - 1] == b',' {
        return (left - 1, byte + len);
    }
    (byte, ws_comma_right(line, byte + len))
}

fn partial_enable_cop_range(
    source: &SourceFile,
    line_no: usize,
    cop: &str,
) -> Option<(usize, usize)> {
    let line_start = source.line_start(line_no)?;
    let directives = crate::parse::directives::cached(source);
    let line = directives.comment_line(source, line_no)?;
    nth_cop_token(&line, cop, 1).map(|(start, end)| {
        let (rs, re) = partial_token_span(&line, start, end - start);
        (line_start + rs, line_start + re)
    })
}

fn entire_enable_line_range(
    source: &SourceFile,
    line_no: usize,
    _line: &str,
) -> Option<(usize, usize)> {
    let start = source.line_start(line_no)?;
    Some((
        start,
        source
            .line_start(line_no + 1)
            .unwrap_or(source.as_bytes().len()),
    ))
}

fn push_enable_fix(
    source: &SourceFile,
    line_no: usize,
    line: &str,
    cop: &str,
    remove_entire: bool,
    entire_fix: &mut bool,
    corr: &mut Vec<Correction>,
) -> bool {
    let range = if remove_entire {
        if *entire_fix {
            return true;
        }
        *entire_fix = true;
        entire_enable_line_range(source, line_no, line)
    } else {
        partial_enable_cop_range(source, line_no, cop)
    };
    let Some((start, end)) = range else {
        return false;
    };
    corr.push(Correction {
        start,
        end,
        replacement: String::new(),
        cop_name: "Lint/RedundantCopEnableDirective",
        cop_index: 0,
        owner: None,
    });
    true
}

fn collect_orphaned<'a>(disabled: &mut Disabled, names: &'a [String]) -> Vec<&'a str> {
    let mut out = Vec::new();
    for name in names {
        if disabled.enable(name) {
            continue;
        }
        out.push(name.as_str());
    }
    out
}

fn report_orphaned_enable(
    cop: &RedundantCopEnableDirective,
    source: &SourceFile,
    line: &str,
    line_no: usize,
    name: &str,
    remove_entire: bool,
    entire_fix: &mut bool,
    corrections: &mut Option<&mut Vec<Correction>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let (start, end) = nth_cop_token(line, name, 1).unwrap_or((0, name.len()));
    let column = crate::parse::source::byte_index_to_column(line, start);
    let display = if name == "all" { "all cops" } else { name };
    let mut diag = cop.diagnostic(
        source,
        line_no,
        column,
        format!("Unnecessary enabling of {display}."),
    );
    diag.highlight_length = end - start;
    if let Some(corr) = corrections.as_deref_mut()
        && push_enable_fix(source, line_no, line, name, remove_entire, entire_fix, corr)
    {
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn check_enable(
    cop: &RedundantCopEnableDirective,
    source: &SourceFile,
    disabled: &mut Disabled,
    line: &str,
    line_no: usize,
    diagnostics: &mut Vec<Diagnostic>,
    mut corrections: Option<&mut Vec<Correction>>,
) {
    let Some(rest) = line.split("# rubocop:enable").nth(1) else {
        return;
    };
    if !line.trim_start().starts_with('#') {
        return;
    }
    let names = directive_names(rest);
    let orphaned = collect_orphaned(disabled, &names);
    if orphaned.is_empty() {
        return;
    }
    let remove_entire = orphaned.len() == names.len();
    let mut entire_fix = false;
    for name in orphaned {
        report_orphaned_enable(
            cop,
            source,
            line,
            line_no,
            name,
            remove_entire,
            &mut entire_fix,
            &mut corrections,
            diagnostics,
        );
    }
}

impl Cop for RedundantCopEnableDirective {
    fn name(&self) -> &'static str {
        "Lint/RedundantCopEnableDirective"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn uses_source_phase(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        tree: &tree_sitter::Tree,
        _code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        // RuboCop seeds disable counts with cops disabled in config.
        let mut disabled = Disabled::new(config_disabled(config));
        let directives = crate::parse::directives::for_source(source, tree);
        for &line_no in directives.comment_lines() {
            let Some(s) = directives.comment_line(source, line_no) else {
                continue;
            };
            disabled.disable_comment(&s);
            check_enable(
                self,
                source,
                &mut disabled,
                &s,
                line_no,
                diagnostics,
                corrections.as_deref_mut(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantCopEnableDirective,
        "cops/lint/redundant_cop_enable_directive"
    );

    #[test]
    fn config_disabled_enable_ok() {
        let mut config = CopConfig::default();
        config.options.insert(
            "ConfigDisabledCops".into(),
            serde_yml::Value::Sequence(vec![serde_yml::Value::String("Layout/LineLength".into())]),
        );
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &RedundantCopEnableDirective,
            b"x = 1\n# rubocop:enable Layout/LineLength\n",
            config,
        );
    }

    fn fixed(src: &[u8]) -> Vec<u8> {
        let source = SourceFile::from_bytes("test.rb", src.to_vec());
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let code_map = CodeMap::from_tree(tree.root_node(), source.as_bytes());
        let mut corrs = Vec::new();
        RedundantCopEnableDirective.check_source(
            &source,
            &tree,
            &code_map,
            &CopConfig::default(),
            &mut Vec::new(),
            Some(&mut corrs),
        );
        crate::correction::CorrectionSet::from_vec(corrs).apply(src)
    }

    #[test]
    fn autocorrect_removes_orphan_enable_line() {
        assert_eq!(
            fixed(b"x = 1\n# rubocop:enable Style/StringLiterals\n"),
            b"x = 1\n"
        );
    }

    #[test]
    fn autocorrect_partial_multi_enable() {
        assert_eq!(
            fixed(b"# rubocop:disable Layout/LineLength\nx = 1\n# rubocop:enable Layout/LineLength, Style/StringLiterals\n"),
            b"# rubocop:disable Layout/LineLength\nx = 1\n# rubocop:enable Layout/LineLength\n"
        );
    }
}
