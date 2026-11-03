use crate::cop::rspec::helpers::{RSPEC_INCLUDE, bare_rspec_call, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::{call_receiver, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct ExpectOutput;
impl Cop for ExpectOutput {
    fn name(&self) -> &'static str {
        "RSpec/ExpectOutput"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["assignment", "operator_assignment"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(left) = node.child_by_field_name("left") else {
            return;
        };
        let variable = node_bytes(source, left);
        if left.kind() != "global_variable"
            || ![b"$stdout".as_slice(), b"$stderr"].contains(&variable)
        {
            return;
        }
        let mut ancestor = node.parent();
        let mut inside = false;
        while let Some(parent) = ancestor {
            if call_block(parent).is_some() {
                if bare_rspec_call(source, parent).is_some_and(is_example_group) {
                    break;
                }
                if call_receiver(parent).is_none()
                    && name(source, parent).is_some_and(crate::cop::rspec::helpers::is_example)
                {
                    inside = true;
                    break;
                }
                if call_receiver(parent).is_none()
                    && name(source, parent).is_some_and(|name| hooks().contains(&name))
                {
                    let args = call_args(parent);
                    inside = args.first().is_none_or(|arg| {
                        arg.kind() == "hash"
                            || arg.kind() == "pair"
                            || symbol_node(*arg)
                                && matches!(symbol(source, *arg), Some(b"each" | b"example"))
                    });
                    break;
                }
            }
            ancestor = parent.parent();
        }
        if !inside {
            return;
        }
        let stream = String::from_utf8_lossy(&variable[1..]);
        report(
            self,
            source,
            left.start_byte(),
            left.end_byte(),
            format!(
                "Use `expect {{ ... }}.to output(...).to_{stream}` instead of mutating ${stream}."
            ),
            diagnostics,
        );
    }
}
