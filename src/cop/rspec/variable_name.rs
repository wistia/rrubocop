use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct VariableName;
impl Cop for VariableName {
    fn name(&self) -> &'static str {
        "RSpec/VariableName"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "identifier"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"let", b"let!", b"subject", b"subject!"]
    }
    fn interested_identifier_names(&self) -> &'static [&'static [u8]] {
        self.interested_call_names()
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<Correction>>,
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
        if variable.kind() == "chained_string" {
            return;
        }
        let Some(value) = crate::cop::ported::static_literal(source, variable) else {
            return;
        };
        if compiled_patterns(source, config, "AllowedPatterns")
            .iter()
            .any(|pattern| pattern.is_match(&value))
        {
            return;
        }
        let style = config.get_str("EnforcedStyle", "snake_case");
        if variable_style(&value, style) {
            return;
        }
        report(
            self,
            source,
            variable.start_byte(),
            variable.end_byte(),
            format!("Use {style} for variable names."),
            diagnostics,
        );
    }
}
