//! Style/EmptyLiteral — Array.new / Hash.new / String.new without args or block.

use tree_sitter::Node;

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EmptyLiteral;

fn const_name<'a>(node: Node<'a>, src: &'a [u8]) -> Option<&'a str> {
    std::str::from_utf8(&src[node.start_byte()..node.end_byte()]).ok()
}

fn has_block(node: Node<'_>) -> bool {
    let mut c = node.walk();
    node.children(&mut c)
        .any(|n| matches!(n.kind(), "block" | "do_block"))
}

fn replacement_for(recv: &str) -> Option<&'static str> {
    match recv {
        "Array" => Some("[]"),
        "Hash" => Some("{}"),
        "String" => Some("''"),
        _ => None,
    }
}

fn method_ident<'a>(node: Node<'a>) -> Option<Node<'a>> {
    node.child_by_field_name("method").or_else(|| {
        let mut c = node.walk();
        node.children(&mut c).find(|n| n.kind() == "identifier")
    })
}

impl Cop for EmptyLiteral {
    fn name(&self) -> &'static str {
        "Style/EmptyLiteral"
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
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let src = source.as_bytes();
        let Some(method) = method_ident(node) else {
            return;
        };
        if const_name(method, src) != Some("new") {
            return;
        }
        let Some(receiver) = node.child_by_field_name("receiver") else {
            return;
        };
        if node.child_by_field_name("arguments").is_some() || has_block(node) {
            return;
        }
        let recv = const_name(receiver, src).unwrap_or("");
        let Some(replacement) = replacement_for(recv) else {
            return;
        };
        Self::report(
            self,
            source,
            node,
            recv,
            replacement,
            diagnostics,
            &mut corrections,
        );
    }
}

impl EmptyLiteral {
    fn report(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        recv: &str,
        replacement: &str,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: &mut Option<&mut Vec<Correction>>,
    ) {
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut diag = self.diagnostic(
            source,
            line,
            col,
            format!("Use `{replacement}` instead of `{recv}.new`."),
        );
        if let Some(corr) = corrections.as_deref_mut() {
            corr.push(Correction {
                start: node.start_byte(),
                end: node.end_byte(),
                replacement: replacement.to_string(),
                cop_name: self.name(),
                cop_index: 0,
                owner: None,
            });
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}
