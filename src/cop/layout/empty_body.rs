//! Shared empty-line checks for Layout/EmptyLinesAround*Body cops.

use tree_sitter::Node;

use crate::cop::Cop;
use crate::cop::shared;
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

struct BodyEdges {
    start_line: usize,
    end_line: usize,
    first_body: usize,
    last_body: usize,
}

fn body_edges(source: &SourceFile, node: Node<'_>) -> Option<BodyEdges> {
    let start_line = shared::node_line(source, node);
    let end_line = shared::end_keyword(node)
        .map(|e| shared::node_line(source, e))
        .unwrap_or_else(|| {
            let (l, _) = source.offset_to_line_col(node.end_byte().saturating_sub(1));
            l
        });
    if end_line <= start_line + 1 {
        return None;
    }
    Some(BodyEdges {
        start_line,
        end_line,
        first_body: start_line + 1,
        last_body: end_line - 1,
    })
}

fn insert_blank(
    cop: &dyn Cop,
    source: &SourceFile,
    line: usize,
    msg: String,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let mut diag = cop.diagnostic(source, line, 0, msg);
    if let Some(corr) = corrections {
        if let Some(off) = source.line_start(line) {
            corr.push(Correction {
                start: off,
                end: off,
                replacement: "\n".into(),
                cop_name: cop.name(),
                cop_index: 0,
                owner: None,
            });
            diag.corrected = true;
        }
    }
    diagnostics.push(diag);
}

fn remove_blank(
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
                owner: None,
            });
            diag.corrected = true;
        }
    }
    diagnostics.push(diag);
}

fn want_empty_edges(
    cop: &dyn Cop,
    source: &SourceFile,
    edges: &BodyEdges,
    label: &str,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let beginning_blank = shared::line_blank(source, edges.first_body);
    let ending_blank = shared::line_blank(source, edges.last_body);
    if !beginning_blank && edges.first_body < edges.end_line {
        insert_blank(
            cop,
            source,
            edges.first_body,
            format!("Empty line missing at {label} body beginning."),
            diagnostics,
            corrections,
        );
    }
    if !ending_blank && edges.last_body > edges.start_line {
        insert_blank(
            cop,
            source,
            edges.end_line,
            format!("Empty line missing at {label} body end."),
            diagnostics,
            corrections,
        );
    }
}

fn no_empty_edges(
    cop: &dyn Cop,
    source: &SourceFile,
    edges: &BodyEdges,
    label: &str,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let beginning_blank = shared::line_blank(source, edges.first_body);
    let ending_blank = shared::line_blank(source, edges.last_body);
    if beginning_blank {
        remove_blank(
            cop,
            source,
            edges.first_body,
            format!("Extra empty line detected at {label} body beginning."),
            diagnostics,
            corrections,
        );
    }
    if ending_blank && edges.last_body != edges.first_body {
        remove_blank(
            cop,
            source,
            edges.last_body,
            format!("Extra empty line detected at {label} body end."),
            diagnostics,
            corrections,
        );
    }
}

/// Enforce empty / no-empty lines around a body for the given label.
pub fn check_body(
    cop: &dyn Cop,
    source: &SourceFile,
    node: Node<'_>,
    want_empty: bool,
    label: &str,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let Some(edges) = body_edges(source, node) else {
        return;
    };
    if want_empty {
        want_empty_edges(cop, source, &edges, label, diagnostics, corrections);
    } else {
        no_empty_edges(cop, source, &edges, label, diagnostics, corrections);
    }
}
