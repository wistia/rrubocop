//! Layout/SpaceInsideRangeLiteral.

use tree_sitter::Node;

use crate::cop::shared;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SpaceInsideRangeLiteral;

fn find_op<'a>(bytes: &[u8], kids: &[Node<'a>]) -> Option<Node<'a>> {
    kids.iter().copied().find(|c| {
        let t = &bytes[c.start_byte()..c.end_byte()];
        t == b".." || t == b"..."
    })
}

fn left_space(
    bytes: &[u8],
    node: Node<'_>,
    kids: &[Node<'_>],
    op_start: usize,
) -> Option<(usize, usize)> {
    let left = node
        .child_by_field_name("left")
        .or_else(|| kids.first().copied())?;
    if left.start_byte() >= op_start {
        return None;
    }
    let le = left.end_byte().min(op_start);
    if shared::has_hspace(bytes, le, op_start) {
        Some((le, op_start))
    } else {
        None
    }
}

fn right_space(
    bytes: &[u8],
    node: Node<'_>,
    kids: &[Node<'_>],
    op_end: usize,
) -> Option<(usize, usize)> {
    let right = node
        .child_by_field_name("right")
        .or_else(|| kids.last().copied())?;
    if right.start_byte() < op_end {
        return None;
    }
    let rs = right.start_byte();
    let between = &bytes[op_end..rs];
    let pure_nl = between.starts_with(b"\n") || between.starts_with(b"\r\n");
    if !pure_nl && shared::has_hspace(bytes, op_end, rs) {
        Some((op_end, rs))
    } else {
        None
    }
}

fn collect_ranges(bytes: &[u8], node: Node<'_>) -> Vec<(usize, usize)> {
    let mut cur = node.walk();
    let kids: Vec<_> = node.children(&mut cur).collect();
    let Some(op) = find_op(bytes, &kids) else {
        return Vec::new();
    };
    let mut ranges = Vec::new();
    if let Some(r) = left_space(bytes, node, &kids, op.start_byte()) {
        ranges.push(r);
    }
    if let Some(r) = right_space(bytes, node, &kids, op.end_byte()) {
        ranges.push(r);
    }
    ranges
}

impl Cop for SpaceInsideRangeLiteral {
    fn name(&self) -> &'static str {
        "Layout/SpaceInsideRangeLiteral"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["range"]
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
        let ranges = collect_ranges(source.as_bytes(), node);
        if ranges.is_empty() {
            return;
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut diag = self.diagnostic(source, line, col, "Space inside range literal.".into());
        if let Some(corr) = corrections.as_mut() {
            for (s, e) in ranges {
                corr.push(Correction {
                    start: s,
                    end: e,
                    replacement: String::new(),
                    cop_name: self.name(),
                    cop_index: 0,
                    owner: None,
                });
            }
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}
