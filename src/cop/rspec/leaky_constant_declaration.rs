//! RSpec/LeakyConstantDeclaration — constants/classes/modules inside example groups.

use tree_sitter::Node;

use crate::cop::rspec::helpers::{RSPEC_INCLUDE, inside_spec_group};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::is_const_assign;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct LeakyConstantDeclaration;

fn offense_message(kind: &str) -> &'static str {
    match kind {
        "class" => "Stub class constant instead of declaring explicitly.",
        "module" => "Stub module constant instead of declaring explicitly.",
        _ => "Stub constant instead of declaring explicitly.",
    }
}

impl Cop for LeakyConstantDeclaration {
    fn name(&self) -> &'static str {
        "RSpec/LeakyConstantDeclaration"
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &[
            "assignment",
            "class",
            "module",
            "constant",
            "scope_resolution",
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let kind = node.kind();
        if kind == "assignment" && !is_const_assign(node) {
            return;
        }
        if matches!(kind, "constant" | "scope_resolution") {
            let Some(parent) = node.parent() else {
                return;
            };
            if !(parent.kind() == "operator_assignment"
                && parent.child_by_field_name("left") == Some(node)
                || parent.kind() == "left_assignment_list")
            {
                return;
            }
        }
        if !inside_spec_group(source, node) {
            return;
        }
        report(
            self,
            source,
            node.start_byte(),
            node.end_byte(),
            offense_message(kind).into(),
            diagnostics,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        LeakyConstantDeclaration,
        "cops/rspec/leaky_constant_declaration"
    );
}
