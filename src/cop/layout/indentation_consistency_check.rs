//! Check routines for IndentationConsistency.

use tree_sitter::Node;

use crate::cop::Cop;
use crate::cop::layout::indentation_consistency_util as util;
use crate::cop::shared;
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

fn fix_indent(
    source: &SourceFile,
    offset: usize,
    base: usize,
    cop_name: &'static str,
    corr: &mut Vec<Correction>,
) -> bool {
    let (line, _) = source.offset_to_line_col(offset);
    let Some(ls) = source.line_start(line) else {
        return false;
    };
    let cur = shared::line_indent(source, offset);
    if cur == base {
        return false;
    }
    corr.push(Correction {
        start: ls,
        end: ls + cur,
        replacement: " ".repeat(base),
        cop_name,
        cop_index: 0,
        owner: None,
    });
    true
}

fn report(
    cop: &dyn Cop,
    source: &SourceFile,
    node: Node<'_>,
    base: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (l, mut c) = source.offset_to_line_col(node.start_byte());
    if l == 1 && source.as_bytes().starts_with(&util::UTF8_BOM) {
        c = c.saturating_sub(1);
    }
    let mut diag = cop.diagnostic(source, l, c, "Inconsistent indentation detected.".into());
    if let Some(corr) = corrections {
        if fix_indent(source, node.start_byte(), base, cop.name(), corr) {
            diag.corrected = true;
        }
    }
    diagnostics.push(diag);
}

fn continues_range(source: &SourceFile, kids: &[Node<'_>], index: usize) -> bool {
    index > 0
        && kids[index].kind() == "parenthesized_statements"
        && shared::node_text(source, kids[index - 1])
            .trim_end()
            .ends_with("..")
}

fn check_flat(
    cop: &dyn Cop,
    source: &SourceFile,
    kids: &[Node<'_>],
    base: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let mut prev_end = 0usize;
    for (i, &k) in kids.iter().enumerate() {
        let line = shared::node_line(source, k);
        let col = util::display_col(source, k.start_byte());
        let same_expression = (i > 0 && line == prev_end) || continues_range(source, kids, i);
        prev_end = util::end_line(source, k);
        if same_expression {
            continue;
        }
        if !util::begins_its_line(source, k.start_byte()) {
            continue;
        }
        if col != base {
            report(cop, source, k, base, diagnostics, corrections);
        }
    }
}

fn check_normal(
    cop: &dyn Cop,
    source: &SourceFile,
    n: Node<'_>,
    kids: Vec<Node<'_>>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let base_from_mod = util::base_column_for_normal(source, &kids, util::parent_column(source, n));
    let filtered: Vec<_> = kids
        .into_iter()
        .filter(|k| !util::is_bare_access_modifier(source, *k))
        .collect();
    if filtered.is_empty() || (filtered.len() < 2 && base_from_mod.is_none()) {
        return;
    }
    check_flat(
        cop,
        source,
        &filtered,
        base_from_mod.unwrap_or_else(|| util::display_col(source, filtered[0].start_byte())),
        diagnostics,
        corrections,
    );
}

fn check_sections(
    cop: &dyn Cop,
    source: &SourceFile,
    kids: &[Node<'_>],
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let mut sections: Vec<Vec<Node<'_>>> = vec![vec![]];
    for &k in kids {
        if util::is_bare_access_modifier(source, k) {
            sections.push(vec![]);
        } else {
            sections.last_mut().unwrap().push(k);
        }
    }
    for section in &sections {
        if section.len() < 2 {
            continue;
        }
        check_flat(
            cop,
            source,
            section,
            util::display_col(source, section[0].start_byte()),
            diagnostics,
            corrections,
        );
    }
}

pub fn check_list(
    cop: &dyn Cop,
    source: &SourceFile,
    n: Node<'_>,
    indented: bool,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    if !util::is_stmt_list(n.kind()) {
        return;
    }
    let kids = util::stmt_kids(source, n);
    if indented {
        if kids.len() < 2 {
            return;
        }
        check_sections(cop, source, &kids, diagnostics, corrections);
    } else {
        check_normal(cop, source, n, kids, diagnostics, corrections);
    }
}
