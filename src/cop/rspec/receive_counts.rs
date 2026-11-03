use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct ReceiveCounts;
impl Cop for ReceiveCounts {
    fn name(&self) -> &'static str {
        "RSpec/ReceiveCounts"
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
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if safe_send(source, node) {
            return;
        }
        if name(source, node) != Some(b"times") || !call_args(node).is_empty() {
            return;
        }
        let Some(counter) = call_receiver(node) else {
            return;
        };
        let Some(method) = name(source, counter) else {
            return;
        };
        if safe_send(source, counter)
            || ![b"exactly".as_slice(), b"at_least", b"at_most"].contains(&method)
        {
            return;
        }
        let counts = call_args(counter);
        if counts.len() != 1 || counts[0].kind() != "integer" {
            return;
        }
        let Some(value) = integer_value(source, counts[0]) else {
            return;
        };
        let count = value.to_string();
        let count = count.as_bytes();
        if count != b"1" && count != b"2" {
            return;
        }
        let Some(stub) = call_receiver(counter) else {
            return;
        };
        if !contains_bare_method(source, stub, b"receive") {
            return;
        }
        let Some(dot) = counter.child_by_field_name("operator") else {
            return;
        };
        let ending = if count == b"1" { "once" } else { "twice" };
        let method = String::from_utf8_lossy(method);
        let replacement = if method == "exactly" {
            format!(".{ending}")
        } else {
            format!(".{method}(:{ending})")
        };
        let original =
            String::from_utf8_lossy(&source.as_bytes()[dot.start_byte()..header_end(node)]);
        let index = report(
            self,
            source,
            dot.start_byte(),
            header_end(node),
            format!("Use `{replacement}` instead of `{original}`."),
            diagnostics,
        );
        if preserving_replace(
            source,
            node,
            &mut corrections,
            dot.start_byte(),
            header_end(node),
            replacement,
            self.name(),
        ) {
            diagnostics[index].corrected = true;
        }
    }
}
