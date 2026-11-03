//! Security/YAMLLoad, gated on the Psych default change in Ruby 3.1.

use crate::cop::shared::{call_method_name, call_receiver, method_node, node_bytes, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct YamlLoad;

impl Cop for YamlLoad {
    fn name(&self) -> &'static str {
        "Security/YAMLLoad"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn safe_autocorrect(&self) -> bool {
        false
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if config.get_f64("TargetRubyVersion", 2.7) >= 3.1
            || call_method_name(source, node) != Some(b"load")
            || node
                .child_by_field_name("operator")
                .is_some_and(|op| node_bytes(source, op) == b"&.")
        {
            return;
        }
        let Some(receiver) = call_receiver(node) else {
            return;
        };
        if !matches!(node_bytes(source, receiver), b"YAML" | b"::YAML") {
            return;
        }
        let Some(method) = method_node(node) else {
            return;
        };
        let (line, column) = source.offset_to_line_col(method.start_byte());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Prefer using `YAML.safe_load` over `YAML.load`.".into(),
        );
        diagnostic.highlight_length = 4;
        diagnostic.corrected = push_replace(
            &mut corrections,
            method.start_byte(),
            method.end_byte(),
            "safe_load",
            self.name(),
        );
        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_ruby_specs_and_version_branches() {
        crate::testutil::assert_original_spec_parity(
            &YamlLoad,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/security/yaml_load/parity.json"
            )),
        );
    }
}
