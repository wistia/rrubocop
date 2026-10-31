//! Layout/SpaceAroundMethodCallOperator.

use tree_sitter::Node;

use crate::cop::shared;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SpaceAroundMethodCallOperator;

fn op_span(bytes: &[u8], recv: Node<'_>, method: Node<'_>) -> Option<(usize, usize)> {
    let search = &bytes[recv.end_byte()..method.start_byte()];
    let dot_rel = search.iter().position(|&b| b == b'.')?;
    let dot = recv.end_byte() + dot_rel;
    let op_start = if dot > 0 && bytes[dot - 1] == b'&' {
        dot - 1
    } else {
        dot
    };
    Some((op_start, dot + 1))
}

fn strip_spaces(
    corr: &mut Vec<Correction>,
    cop_name: &'static str,
    recv_end: usize,
    op_start: usize,
    op_end: usize,
    method_start: usize,
    before: bool,
    after: bool,
) {
    if before {
        corr.push(Correction {
            start: recv_end,
            end: op_start,
            replacement: String::new(),
            cop_name,
            cop_index: 0,
            owner: None,
        });
    }
    if after {
        corr.push(Correction {
            start: op_end,
            end: method_start,
            replacement: String::new(),
            cop_name,
            cop_index: 0,
            owner: None,
        });
    }
}

fn report_spaces(
    cop: &dyn Cop,
    source: &SourceFile,
    recv: Node<'_>,
    method: Node<'_>,
    op_start: usize,
    op_end: usize,
    before: bool,
    after: bool,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(op_start);
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        "Avoid using spaces around a method call operator.".into(),
    );
    if let Some(corr) = corrections {
        strip_spaces(
            corr,
            cop.name(),
            recv.end_byte(),
            op_start,
            op_end,
            method.start_byte(),
            before,
            after,
        );
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

impl Cop for SpaceAroundMethodCallOperator {
    fn name(&self) -> &'static str {
        "Layout/SpaceAroundMethodCallOperator"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let _ = config;
        let bytes = source.as_bytes();
        let Some(recv) = node.child_by_field_name("receiver") else {
            return;
        };
        let Some(method) = node.child_by_field_name("method") else {
            return;
        };
        let Some((op_start, op_end)) = op_span(bytes, recv, method) else {
            return;
        };
        let before = shared::only_hspace(bytes, recv.end_byte(), op_start);
        let after = shared::only_hspace(bytes, op_end, method.start_byte());
        if !(before || after) {
            return;
        }
        report_spaces(
            self,
            source,
            recv,
            method,
            op_start,
            op_end,
            before,
            after,
            diagnostics,
            &mut corrections,
        );
    }
}
