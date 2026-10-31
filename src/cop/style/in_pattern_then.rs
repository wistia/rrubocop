//! Style/InPatternThen — no `then` on `in` pattern.

use tree_sitter::Node;

use crate::cop::shared::node_bytes;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct InPatternThen;

impl Cop for InPatternThen {
    fn name(&self) -> &'static str {
        "Style/InPatternThen"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["in"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(then_kw) = find_then(source, node) else {
            return;
        };
        report(self, source, then_kw, diagnostics, &mut corrections);
    }
}

fn find_then<'a>(source: &SourceFile, node: Node<'a>) -> Option<Node<'a>> {
    let mut cur = node.walk();
    node.children(&mut cur)
        .find(|child| node_bytes(source, *child) == b"then")
}

fn report(
    cop: &InPatternThen,
    source: &SourceFile,
    then_kw: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(then_kw.start_byte());
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        "Do not use `then` with `in`-pattern matching.".to_string(),
    );
    if let Some(corr) = corrections.as_mut() {
        corr.push(Correction {
            start: skip_spaces_before(source.as_bytes(), then_kw.start_byte()),
            end: then_kw.end_byte(),
            replacement: String::new(),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn skip_spaces_before(src: &[u8], mut pos: usize) -> usize {
    while pos > 0 && matches!(src[pos - 1], b' ' | b'\t') {
        pos -= 1;
    }
    pos
}
