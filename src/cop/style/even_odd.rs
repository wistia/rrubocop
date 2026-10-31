//! Style/EvenOdd — prefer even?/odd? over % 2.

use tree_sitter::Node;

use crate::cop::shared::node_bytes;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EvenOdd;

impl Cop for EvenOdd {
    fn name(&self) -> &'static str {
        "Style/EvenOdd"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["binary"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some((method, recv)) = even_odd_fix(source, node) else {
            return;
        };
        report(
            self,
            source,
            node,
            method,
            &recv,
            diagnostics,
            &mut corrections,
        );
    }
}

fn even_odd_fix(source: &SourceFile, node: Node<'_>) -> Option<(&'static str, String)> {
    let (op, left, right_node) = cmp_parts(source, node)?;
    let (recv_node, right) = mod2_parts(source, left, right_node)?;
    let want_even = (right == b"0" && op == b"==") || (right == b"1" && op == b"!=");
    let method = if want_even { "even?" } else { "odd?" };
    Some((
        method,
        String::from_utf8_lossy(node_bytes(source, recv_node)).into_owned(),
    ))
}

fn cmp_parts<'a>(source: &'a SourceFile, node: Node<'a>) -> Option<(&'a [u8], Node<'a>, Node<'a>)> {
    let mut cur = node.walk();
    let kids: Vec<_> = node.children(&mut cur).collect();
    if kids.len() < 3 {
        return None;
    }
    let op = node_bytes(source, kids[1]);
    if op != b"==" && op != b"!=" {
        return None;
    }
    Some((op, kids[0], kids[2]))
}

fn mod2_parts<'a>(
    source: &'a SourceFile,
    left: Node<'a>,
    right_node: Node<'a>,
) -> Option<(Node<'a>, &'a [u8])> {
    if left.kind() != "binary" {
        return None;
    }
    let mut lcur = left.walk();
    let lkids: Vec<_> = left.children(&mut lcur).collect();
    if lkids.len() < 3
        || node_bytes(source, lkids[1]) != b"%"
        || node_bytes(source, lkids[2]) != b"2"
    {
        return None;
    }
    let right = node_bytes(source, right_node);
    if right != b"0" && right != b"1" {
        return None;
    }
    Some((lkids[0], right))
}

fn report(
    cop: &EvenOdd,
    source: &SourceFile,
    node: Node<'_>,
    method: &str,
    recv: &str,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(node.start_byte());
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        format!("Replace with `Integer#{method}`."),
    );
    if let Some(corr) = corrections.as_mut() {
        corr.push(Correction {
            start: node.start_byte(),
            end: node.end_byte(),
            replacement: format!("{recv}.{method}"),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}
