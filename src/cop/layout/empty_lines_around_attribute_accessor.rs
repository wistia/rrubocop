//! Layout/EmptyLinesAroundAttributeAccessor.

use tree_sitter::Node;

use crate::cop::shared;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EmptyLinesAroundAttributeAccessor;

const DEFAULT_ALLOWED: &[&str] = &["alias_method", "public", "protected", "private"];

fn is_attr(name: &[u8]) -> bool {
    matches!(
        name,
        b"attr_reader" | b"attr_writer" | b"attr_accessor" | b"attr"
    )
}

fn allowed_methods(config: &CopConfig) -> Vec<String> {
    match config.options.get("AllowedMethods") {
        Some(serde_yml::Value::Sequence(seq)) => seq
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        _ => DEFAULT_ALLOWED.iter().map(|s| (*s).to_string()).collect(),
    }
}

fn next_code_sibling(node: Node<'_>) -> Option<Node<'_>> {
    let mut n = node.next_named_sibling();
    while let Some(sib) = n {
        if sib.kind() != "comment" {
            return Some(sib);
        }
        n = sib.next_named_sibling();
    }
    None
}

fn is_allowed_successor(source: &SourceFile, node: Node<'_>, config: &CopConfig) -> bool {
    if node.kind() == "alias" && config.get_bool("AllowAliasSyntax", true) {
        return true;
    }
    let Some(name) = shared::call_method_name(source, node) else {
        return false;
    };
    is_attr(name) || allowed_methods(config).iter().any(|m| m.as_bytes() == name)
}

fn in_conditional(node: Node<'_>) -> bool {
    matches!(
        node.parent().map(|p| p.kind()),
        Some("if" | "unless" | "then" | "else" | "elsif" | "when" | "rescue" | "ensure")
    )
}

fn gap_after(source: &SourceFile, node: Node<'_>) -> Option<usize> {
    let (end_line, _) = source.offset_to_line_col(node.end_byte().saturating_sub(1));
    let gap = end_line + 1;
    (!shared::line_blank(source, gap) && source.line_start(gap).is_some()).then_some(gap)
}

fn attr_needs_blank(source: &SourceFile, node: Node<'_>, config: &CopConfig) -> Option<usize> {
    let name = shared::call_method_name(source, node)?;
    if !is_attr(name) || shared::call_receiver(node).is_some() || in_conditional(node) {
        return None;
    }
    let next = next_code_sibling(node)?;
    if is_allowed_successor(source, next, config) {
        return None;
    }
    gap_after(source, node)
}

impl Cop for EmptyLinesAroundAttributeAccessor {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundAttributeAccessor"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "command"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(gap_line) = attr_needs_blank(source, node, config) else {
            return;
        };
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut diag = self.diagnostic(
            source,
            line,
            col,
            "Add an empty line after attribute accessor.".to_string(),
        );
        if let Some(corr) = corrections.as_deref_mut() {
            if let Some(offset) = source.line_start(gap_line) {
                corr.push(Correction {
                    start: offset,
                    end: offset,
                    replacement: "\n".into(),
                    cop_name: self.name(),
                    cop_index: 0,
                    owner: None,
                });
                diag.corrected = true;
            }
        }
        diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        EmptyLinesAroundAttributeAccessor,
        "cops/layout/empty_lines_around_attribute_accessor"
    );
}
