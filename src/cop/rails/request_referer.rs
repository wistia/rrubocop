//! Rails/RequestReferer — breadth-first tree-sitter port.

use tree_sitter::Node;

use crate::cop::shared::{call_method_name, method_node, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RequestReferer;

fn replacement<'a>(source: &SourceFile, node: Node<'_>, config: &'a CopConfig) -> Option<&'a str> {
    let receiver = node.child_by_field_name("receiver")?;
    if crate::cop::shared::node_bytes(source, receiver) != b"request" {
        return None;
    }
    let prefer = config.get_str("EnforcedStyle", "referer");
    let wrong = if prefer == "referer" {
        b"referrer".as_slice()
    } else {
        b"referer".as_slice()
    };
    (call_method_name(source, node) == Some(wrong)).then_some(prefer)
}

impl Cop for RequestReferer {
    fn name(&self) -> &'static str {
        "Rails/RequestReferer"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "command"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(prefer) = replacement(source, node, config) else {
            return;
        };
        let wrong = if prefer == "referer" {
            "referrer"
        } else {
            "referer"
        };
        let meth = method_node(node).unwrap_or(node);
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut diag = self.diagnostic(
            source,
            line,
            col,
            format!("Use `request.{prefer}` instead of `request.{wrong}`."),
        );
        if push_replace(
            &mut corrections,
            meth.start_byte(),
            meth.end_byte(),
            prefer,
            self.name(),
        ) {
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}
