//! Layout/EmptyLinesAroundExceptionHandlingKeywords.

use tree_sitter::{Node, Tree};

use crate::cop::shared;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct EmptyLinesAroundExceptionHandlingKeywords;

fn remove_blank_line(
    cop: &dyn Cop,
    source: &SourceFile,
    line: usize,
    msg: String,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let mut diag = cop.diagnostic(source, line, 0, msg);
    if let Some(corr) = corrections {
        if let Some(s) = source.line_start(line) {
            corr.push(Correction {
                start: s,
                end: source.line_start(line + 1).unwrap_or(s),
                replacement: String::new(),
                cop_name: cop.name(),
                cop_index: 0,
            });
            diag.corrected = true;
        }
    }
    diagnostics.push(diag);
}

fn check_before(
    cop: &dyn Cop,
    source: &SourceFile,
    line: usize,
    kw: &str,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    if line <= 1 || !shared::line_blank(source, line - 1) {
        return;
    }
    remove_blank_line(
        cop,
        source,
        line - 1,
        format!("Extra empty line detected before the `{kw}`."),
        diagnostics,
        corrections,
    );
}

fn check_after(
    cop: &dyn Cop,
    source: &SourceFile,
    n: Node<'_>,
    line: usize,
    kw: &str,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    if !shared::line_blank(source, line + 1) {
        return;
    }
    let (end_l, _) = source.offset_to_line_col(n.end_byte().saturating_sub(1));
    if end_l <= line + 1 {
        return;
    }
    remove_blank_line(
        cop,
        source,
        line + 1,
        format!("Extra empty line detected after the `{kw}`."),
        diagnostics,
        corrections,
    );
}

fn exception_else(n: Node<'_>) -> bool {
    if n.kind() == "else" {
        let Some(parent) = n.parent() else {
            return false;
        };
        if !matches!(parent.kind(), "begin" | "body_statement") {
            return false;
        }
        let mut cur = parent.walk();
        if !parent
            .named_children(&mut cur)
            .any(|c| c.kind() == "rescue")
        {
            return false;
        }
    }
    true
}

fn check_kw(
    cop: &dyn Cop,
    source: &SourceFile,
    n: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    if !matches!(n.kind(), "rescue" | "ensure" | "else") {
        return;
    }
    // RuboCop only flags `else` inside begin/rescue — not if/case else.
    if !exception_else(n) {
        return;
    }
    if shared::node_col(source, n) != shared::line_indent(source, n.start_byte()) {
        return;
    }
    let line = shared::node_line(source, n);
    let kw = n.kind();
    check_before(cop, source, line, kw, diagnostics, corrections);
    check_after(cop, source, n, line, kw, diagnostics, corrections);
}

impl Cop for EmptyLinesAroundExceptionHandlingKeywords {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundExceptionHandlingKeywords"
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
        tree: &Tree,
        code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let _ = (code_map, config);
        shared::for_each_descendant(tree.root_node(), |n| {
            check_kw(self, source, n, diagnostics, &mut corrections);
        });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn inline_rescue_has_no_blank_line_offense() {
        crate::testutil::assert_cop_no_offenses_full(
            &super::EmptyLinesAroundExceptionHandlingKeywords,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/layout/empty_lines_around_exception_handling_keywords/no_offense.rb"
            )),
        );
    }
}
