//! Style/UnlessElse — rewrite `unless`/`else` as `if`/`else` with swapped bodies.

use tree_sitter::Node;

use crate::cop::shared::child_kind;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct UnlessElse;

impl Cop for UnlessElse {
    fn name(&self) -> &'static str {
        "Style/UnlessElse"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["unless"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(else_n) = child_kind(node, "else") else {
            return;
        };
        report(self, source, node, else_n, diagnostics, &mut corrections);
    }
}

fn report(
    cop: &UnlessElse,
    source: &SourceFile,
    node: Node<'_>,
    else_n: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (kw_start, kw_end) = unless_kw_range(source, node);
    let (line, col) = source.offset_to_line_col(kw_start);
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        "Do not use `unless` with `else`. Rewrite these with the positive case first.".to_string(),
    );
    if let Some(corr) = corrections.as_mut() {
        push_corr(corr, kw_start, kw_end, "if".into(), cop.name());
        push_body_swap(cop, source, node, else_n, corr);
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn unless_kw_range(source: &SourceFile, node: Node<'_>) -> (usize, usize) {
    find_unless_kw(source, node)
        .map(|k| (k.start_byte(), k.end_byte()))
        .unwrap_or((node.start_byte(), node.start_byte() + 6))
}

fn find_unless_kw<'a>(source: &SourceFile, node: Node<'a>) -> Option<Node<'a>> {
    let mut cur = node.walk();
    if let Some(k) = node
        .children(&mut cur)
        .find(|c| !c.is_named() && c.kind() == "unless")
    {
        return Some(k);
    }
    let mut c2 = node.walk();
    node.children(&mut c2)
        .find(|c| &source.as_bytes()[c.start_byte()..c.end_byte()] == b"unless")
}

fn push_body_swap(
    cop: &UnlessElse,
    source: &SourceFile,
    node: Node<'_>,
    else_n: Node<'_>,
    corr: &mut Vec<Correction>,
) {
    let Some((u, e)) = body_ranges(source, node, else_n) else {
        return;
    };
    swap_ranges(corr, source.as_bytes(), u, e, cop.name());
}

fn body_ranges(
    source: &SourceFile,
    node: Node<'_>,
    else_n: Node<'_>,
) -> Option<((usize, usize), (usize, usize))> {
    let then_n = then_body(node)?;
    let src = source.as_bytes();
    let else_kw_end = child_kind(else_n, "else")
        .map(|k| k.end_byte())
        .unwrap_or(else_n.start_byte());
    let u = trim_end(src, then_n.start_byte(), then_n.end_byte());
    valid_pair(u, trim_end(src, else_kw_end, else_n.end_byte()))
}

fn then_body(node: Node<'_>) -> Option<Node<'_>> {
    node.child_by_field_name("consequence")
        .or_else(|| node.child_by_field_name("body"))
}

fn valid_pair(u: (usize, usize), e: (usize, usize)) -> Option<((usize, usize), (usize, usize))> {
    if u.1 < u.0 || e.1 < e.0 {
        None
    } else {
        Some((u, e))
    }
}

fn swap_ranges(
    corr: &mut Vec<Correction>,
    src: &[u8],
    u: (usize, usize),
    e: (usize, usize),
    cop: &'static str,
) {
    let unless_txt = String::from_utf8_lossy(&src[u.0..u.1]).into_owned();
    push_corr(
        corr,
        u.0,
        u.1,
        String::from_utf8_lossy(&src[e.0..e.1]).into_owned(),
        cop,
    );
    push_corr(corr, e.0, e.1, unless_txt, cop);
}

fn push_corr(
    corr: &mut Vec<Correction>,
    start: usize,
    end: usize,
    replacement: String,
    cop: &'static str,
) {
    corr.push(Correction {
        start,
        end,
        replacement,
        cop_name: cop,
        cop_index: 0,
        owner: None,
    });
}

fn trim_end(src: &[u8], start: usize, mut end: usize) -> (usize, usize) {
    while end > start && matches!(src[end - 1], b' ' | b'\t' | b'\n' | b'\r') {
        end -= 1;
    }
    (start, end)
}
