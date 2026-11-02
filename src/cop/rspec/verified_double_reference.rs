use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::{call_receiver, node_text};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct VerifiedDoubleReference;
impl Cop for VerifiedDoubleReference {
    fn name(&self) -> &'static str {
        "RSpec/VerifiedDoubleReference"
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
    fn safe_autocorrect(&self) -> bool {
        false
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if call_receiver(node).is_some()
            || !name(source, node).is_some_and(|method| {
                [
                    b"class_double".as_slice(),
                    b"class_spy",
                    b"instance_double",
                    b"instance_spy",
                    b"mock_model",
                    b"object_double",
                    b"object_spy",
                    b"stub_model",
                ]
                .contains(&method)
            })
        {
            return;
        }
        let Some(reference) = call_args(node).first().copied() else {
            return;
        };
        let style = config.get_str("EnforcedStyle", "constant");
        let current = match reference.kind() {
            "string" => "string",
            "constant" | "scope_resolution" => "constant",
            _ => return,
        };
        if current == style || current == "string" && symbol(source, reference).is_none() {
            return;
        }
        let index = report(
            self,
            source,
            reference.start_byte(),
            reference.end_byte(),
            format!("Use a {style} class reference for verified doubles."),
            diagnostics,
        );
        let text = node_text(source, reference);
        let replacement = if style == "string" {
            format!("'{text}'")
        } else {
            text[1..text.len() - 1].to_string()
        };
        if preserving_replace(
            source,
            node,
            &mut corrections,
            reference.start_byte(),
            reference.end_byte(),
            replacement,
            self.name(),
        ) {
            diagnostics[index].corrected = true;
        }
    }
}
