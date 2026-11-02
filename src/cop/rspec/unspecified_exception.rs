use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::{argument_nodes, call_receiver};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct UnspecifiedException;
impl Cop for UnspecifiedException {
    fn name(&self) -> &'static str {
        "RSpec/UnspecifiedException"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "identifier"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<Correction>>,
    ) {
        if !matches!(
            name(source, node),
            Some(b"raise_error" | b"raise_exception")
        ) || !argument_nodes(node).is_empty()
            || call_block(node).is_some()
        {
            return;
        }
        let mut parent = node.parent();
        while let Some(ancestor) = parent {
            if name(source, ancestor) == Some(b"to") {
                if let Some(receiver) = call_receiver(ancestor) {
                    if bare(source, receiver, b"expect") && call_block(receiver).is_some() {
                        if call_block(ancestor)
                            .and_then(|b| b.child_by_field_name("parameters"))
                            .is_none()
                        {
                            report(
                                self,
                                source,
                                node.start_byte(),
                                header_end(node),
                                "Specify the exception being captured".into(),
                                diagnostics,
                            );
                        }
                        return;
                    }
                }
            }
            parent = ancestor.parent();
        }
    }
}
