//! Style/DefWithParentheses — empty arg list needs ().

use tree_sitter::Node;

use crate::cop::shared::node_bytes;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct DefWithParentheses;

impl Cop for DefWithParentheses {
    fn name(&self) -> &'static str {
        "Style/DefWithParentheses"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["method", "singleton_method"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some((start, end)) = empty_parens_range(source, node) else {
            return;
        };
        report(self, source, start, end, diagnostics, &mut corrections);
    }
}

fn report(
    cop: &DefWithParentheses,
    source: &SourceFile,
    start: usize,
    end: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(start);
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        "Omit parentheses for method definitions with no arguments.".to_string(),
    );
    if let Some(corr) = corrections.as_mut() {
        corr.push(Correction {
            start,
            end,
            replacement: String::new(),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn empty_parens_range(source: &SourceFile, node: Node<'_>) -> Option<(usize, usize)> {
    let open = find_open_paren(source, node)?;
    let close = next_sibling_after(node, open)?;
    if node_bytes(source, close) != b")" {
        return None;
    }
    Some((open.start_byte(), close.end_byte()))
}

fn find_open_paren<'a>(source: &SourceFile, node: Node<'a>) -> Option<Node<'a>> {
    let mut cur = node.walk();
    node.children(&mut cur)
        .find(|c| node_bytes(source, *c) == b"(")
}

fn next_sibling_after<'a>(parent: Node<'a>, after: Node<'a>) -> Option<Node<'a>> {
    let mut cur = parent.walk();
    let mut seen = false;
    for c in parent.children(&mut cur) {
        if seen {
            return Some(c);
        }
        if c.id() == after.id() {
            seen = true;
        }
    }
    None
}
