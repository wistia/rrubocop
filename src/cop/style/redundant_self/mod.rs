//! Style/RedundantSelf — avoid unnecessary self.

mod assign_names;
mod guards;
mod redundant;
mod shadow;

use tree_sitter::Node;

use crate::cop::shared::{call_method_name, call_receiver};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantSelf;

impl Cop for RedundantSelf {
    fn name(&self) -> &'static str {
        "Style/RedundantSelf"
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
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let Some(recv) = call_receiver(node) else {
            return;
        };
        if node
            .child_by_field_name("operator")
            .is_some_and(|n| crate::cop::shared::node_bytes(source, n) == b"&.")
        {
            return;
        }
        if recv.kind() != "self" {
            return;
        }
        if guards::call_is_assign_lhs(node) || guards::call_is_index_assign_recv(node) {
            return;
        }
        if !redundant::self_is_redundant(source, node) {
            return;
        }
        if self_assign_shadows(source, node) {
            return;
        }
        report(self, source, node, recv, diagnostics, &mut corrections);
    }
}

fn self_assign_shadows(source: &SourceFile, node: Node<'_>) -> bool {
    let Some(method) = call_method_name(source, node) else {
        return false;
    };
    let bare = method.strip_suffix(b"=").unwrap_or(method);
    assign_names::self_assign_names_in_enclosing_type(source, node)
        .iter()
        .any(|n| n.as_slice() == bare)
}

fn report(
    cop: &RedundantSelf,
    source: &SourceFile,
    node: Node<'_>,
    recv: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(recv.start_byte());
    let mut diag = cop.diagnostic(source, line, col, "Redundant `self` detected.".to_string());
    if let Some(corr) = corrections.as_mut() {
        corr.push(Correction {
            start: recv.start_byte(),
            end: node
                .child_by_field_name("method")
                .unwrap_or(node)
                .start_byte(),
            replacement: String::new(),
            cop_name: cop.name(),
            cop_index: 0,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}
