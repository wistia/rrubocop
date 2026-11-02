use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::{call_receiver, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct VariableDefinition;
impl Cop for VariableDefinition {
    fn name(&self) -> &'static str {
        "RSpec/VariableDefinition"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "identifier"]
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if safe_send(source, node) {
            return;
        }
        if call_receiver(node).is_some()
            || !matches!(
                name(source, node),
                Some(b"subject" | b"subject!" | b"let" | b"let!")
            )
            || !inside_root_spec_group(source, node)
        {
            return;
        }
        let Some(variable) = call_args(node).first().copied() else {
            return;
        };
        let style = config.get_str("EnforcedStyle", "symbols");
        let value = crate::cop::ported::static_literal(source, variable);
        let replacement = if style == "symbols" && variable.kind() == "string" {
            let Some(value) = value else {
                return;
            };
            ruby_symbol(&value)
        } else if style == "strings" && symbol_node(variable) {
            if variable.kind() == "delimited_symbol" && value.is_none() {
                String::from_utf8_lossy(
                    node_bytes(source, variable)
                        .strip_prefix(b":")
                        .unwrap_or_default(),
                )
                .into_owned()
            } else {
                let Some(value) = value else {
                    return;
                };
                ruby_string(&value)
            }
        } else {
            return;
        };
        let index = report(
            self,
            source,
            variable.start_byte(),
            variable.end_byte(),
            format!("Use {style} for variable names."),
            diagnostics,
        );
        if preserving_replace(
            source,
            node,
            &mut corrections,
            variable.start_byte(),
            variable.end_byte(),
            replacement,
            self.name(),
        ) {
            diagnostics[index].corrected = true;
        }
    }
}
