use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct ClassCheck;
impl Cop for ClassCheck {
    fn name(&self) -> &'static str {
        "RSpec/ClassCheck"
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
        if safe_send(source, node) || call_receiver(node).is_some() {
            return;
        }
        let Some(method) = name(source, node) else {
            return;
        };
        let style = config.get_str("EnforcedStyle", "be_a");
        let wrong = if style == "be_a" {
            matches!(method, b"be_kind_of" | b"be_a_kind_of")
        } else {
            matches!(method, b"be_a" | b"be_an")
        };
        if !wrong {
            return;
        }
        let matcher = node;
        let replacement = style;
        let message = format!(
            "Prefer `{style}` over `{}`.",
            String::from_utf8_lossy(method)
        );
        let target = selector(matcher);
        let index = report(
            self,
            source,
            target.start_byte(),
            target.end_byte(),
            message,
            diagnostics,
        );
        if preserving_replace(
            source,
            matcher,
            &mut corrections,
            target.start_byte(),
            target.end_byte(),
            replacement,
            self.name(),
        ) {
            diagnostics[index].corrected = true;
        }
    }
}
