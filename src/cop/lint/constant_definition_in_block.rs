//! Lint/ConstantDefinitionInBlock — constants/classes/modules inside any block.

use tree_sitter::Node;

use crate::cop::shared::{call_method_name, is_const_assign};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ConstantDefinitionInBlock;

fn method_allowed(name: &[u8], config: &CopConfig) -> bool {
    match config.options.get("AllowedMethods") {
        Some(serde_yml::Value::Sequence(items)) => items
            .iter()
            .any(|v| v.as_str().is_some_and(|s| s.as_bytes() == name)),
        Some(serde_yml::Value::String(s)) => s.as_bytes() == name,
        None => name == b"enums",
        _ => false,
    }
}

/// Nearest enclosing block: skip when its method is in `AllowedMethods`.
fn offense_in_block(source: &SourceFile, node: Node<'_>, config: &CopConfig) -> bool {
    let mut p = node.parent();
    while let Some(cur) = p {
        if matches!(cur.kind(), "do_block" | "block") {
            if let Some(name) = cur.parent().and_then(|call| call_method_name(source, call)) {
                if method_allowed(name, config) {
                    return false;
                }
            }
            return true;
        }
        if !matches!(cur.kind(), "body_statement" | "block_body") {
            return false;
        }
        p = cur.parent();
    }
    false
}

impl Cop for ConstantDefinitionInBlock {
    fn name(&self) -> &'static str {
        "Lint/ConstantDefinitionInBlock"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["assignment", "class", "module"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if node.kind() == "assignment"
            && (!is_const_assign(node)
                || node
                    .child_by_field_name("left")
                    .is_some_and(|n| n.kind() == "scope_resolution"))
        {
            return;
        }
        if !offense_in_block(source, node, config) {
            return;
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        diagnostics.push(self.diagnostic(
            source,
            line,
            col,
            "Do not define constants this way within a block.".into(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ConstantDefinitionInBlock,
        "cops/lint/constant_definition_in_block"
    );
}
