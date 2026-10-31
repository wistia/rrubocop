//! Style/SignalException — prefer raise or fail.

use tree_sitter::Node;

use crate::cop::shared::call_method_name;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SignalException;

impl Cop for SignalException {
    fn name(&self) -> &'static str {
        "Style/SignalException"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "command"]
    }

    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"raise", b"fail", b"signal_exception"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(method) = call_method_name(source, node) else {
            return;
        };
        // `obj.fail` is not Kernel#fail (e.g. AASM `withdraw.fail`).
        if crate::cop::shared::call_receiver(node).is_some() {
            return;
        }
        let Some((bad, good)) = map_style(config.get_str("EnforcedStyle", "only_raise"), method)
        else {
            return;
        };
        report(self, source, node, bad, good, diagnostics, &mut corrections);
    }
}

fn map_style(style: &str, method: &[u8]) -> Option<(&'static str, &'static str)> {
    match (style, method) {
        ("only_raise", b"fail") => Some(("fail", "raise")),
        ("only_fail", b"raise") => Some(("raise", "fail")),
        _ => None,
    }
}

fn report(
    cop: &SignalException,
    source: &SourceFile,
    node: Node<'_>,
    bad: &str,
    good: &str,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let meth = node
        .child_by_field_name("method")
        .or_else(|| node.child_by_field_name("name"))
        .unwrap_or(node);
    let (line, col) = source.offset_to_line_col(meth.start_byte());
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        format!("Use `{good}` instead of `{bad}`."),
    );
    if let Some(corr) = corrections.as_mut() {
        corr.push(Correction {
            start: meth.start_byte(),
            end: meth.end_byte(),
            replacement: good.to_string(),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}
