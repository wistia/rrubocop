//! Layout/IndentationStyle.

use tree_sitter::Tree;

use crate::cop::layout::report;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct IndentationStyle;

fn indent_bad(style: &str, indent: &[u8]) -> bool {
    let has_tab = indent.contains(&b'\t');
    let has_space = indent.contains(&b' ');
    if style == "tabs" { has_space } else { has_tab }
}

fn tabs_replacement(indent: &[u8], width: usize) -> String {
    let spaces = indent.iter().filter(|&&b| b == b' ').count() / width.max(1);
    "\t".repeat(spaces + indent.iter().filter(|&&b| b == b'\t').count())
}

fn spaces_replacement(indent: &[u8], width: usize) -> String {
    " ".repeat(
        indent.iter().filter(|&&b| b == b'\t').count() * width
            + indent.iter().filter(|&&b| b == b' ').count(),
    )
}

fn replacement_for(style: &str, indent: &[u8], width: usize) -> String {
    if style == "tabs" {
        tabs_replacement(indent, width)
    } else {
        spaces_replacement(indent, width)
    }
}

fn check_line(
    cop: &dyn Cop,
    source: &SourceFile,
    code_map: &CodeMap,
    style: &str,
    width: usize,
    offset: usize,
    line: &[u8],
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let indent_end = line
        .iter()
        .position(|&b| b != b' ' && b != b'\t')
        .unwrap_or(line.len());
    let indent = &line[..indent_end];
    if indent.is_empty() || !indent_bad(style, indent) || code_map.covers(offset) {
        return;
    }
    let ty = if style == "tabs" { "Space" } else { "Tab" };
    report::report_fix(
        cop,
        source,
        offset,
        format!("{ty} detected in indentation."),
        diagnostics,
        corrections,
        offset,
        offset + indent_end,
        replacement_for(style, indent, width),
    );
}

impl Cop for IndentationStyle {
    fn name(&self) -> &'static str {
        "Layout/IndentationStyle"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn uses_source_phase(&self) -> bool {
        true
    }

    fn needs_code_map(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _tree: &Tree,
        code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "spaces");
        let width = config.get_usize("IndentationWidth", 2);
        let mut offset = 0usize;
        for line in source.lines() {
            check_line(
                self,
                source,
                code_map,
                style,
                width,
                offset,
                line,
                diagnostics,
                &mut corrections,
            );
            offset += line.len() + 1;
        }
    }
}
