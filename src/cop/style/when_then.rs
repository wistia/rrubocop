//! Style/WhenThen — prefer `when x then` over `when x;` on single-line when.

use tree_sitter::Node;

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct WhenThen;

impl Cop for WhenThen {
    fn name(&self) -> &'static str {
        "Style/WhenThen"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["when"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !is_single_line(source, node) || has_then_keyword(source, node) {
            return;
        }
        let Some(semi_offset) = find_semicolon(source, node) else {
            return;
        };
        report(
            self,
            source,
            node,
            semi_offset,
            diagnostics,
            &mut corrections,
        );
    }
}

fn is_single_line(source: &SourceFile, node: Node<'_>) -> bool {
    let (start_line, _) = source.offset_to_line_col(node.start_byte());
    let (end_line, _) =
        source.offset_to_line_col(node.end_byte().saturating_sub(1).max(node.start_byte()));
    start_line == end_line
}

fn has_then_keyword(source: &SourceFile, node: Node<'_>) -> bool {
    let mut cur = node.walk();
    node.children(&mut cur)
        .any(|ch| ch.kind() == "then" && source.as_bytes().get(ch.start_byte()) == Some(&b't'))
}

fn report(
    cop: &WhenThen,
    source: &SourceFile,
    node: Node<'_>,
    semi_offset: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let conditions = conditions_before(source, node, semi_offset);
    let (line, col) = source.offset_to_line_col(semi_offset);
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        format!("Do not use `when {conditions};`. Use `when {conditions} then` instead."),
    );
    if let Some(corr) = corrections {
        corr.push(Correction {
            start: semi_offset,
            end: semi_offset + 1,
            replacement: " then".to_string(),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn conditions_before(source: &SourceFile, node: Node<'_>, semi: usize) -> String {
    let mut cur = node.walk();
    let mut parts = Vec::new();
    for child in node.named_children(&mut cur) {
        if child.start_byte() >= semi {
            break;
        }
        let bytes = &source.as_bytes()[child.start_byte()..child.end_byte()];
        parts.push(String::from_utf8_lossy(bytes).into_owned());
    }
    parts.join(", ")
}

fn find_semicolon(source: &SourceFile, node: Node<'_>) -> Option<usize> {
    let src = source.as_bytes();
    let start = node.start_byte();
    let between = &src[start..node.end_byte()];
    scan_semi(between).map(|i| start + i)
}

fn scan_semi(between: &[u8]) -> Option<usize> {
    let mut in_comment = false;
    for (i, &b) in between.iter().enumerate() {
        match (in_comment, b) {
            (_, b'\n') => in_comment = false,
            (false, b'#') => in_comment = true,
            (false, b';') => return Some(i),
            (false, b' ' | b'\t') => {}
            _ => {}
        }
    }
    None
}
