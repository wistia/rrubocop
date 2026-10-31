//! Style/PreferredHashMethods — key?/value? vs has_key?/has_value?.

use tree_sitter::Node;

use crate::cop::shared::call_method_name;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct PreferredHashMethods;

impl Cop for PreferredHashMethods {
    fn name(&self) -> &'static str {
        "Style/PreferredHashMethods"
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
        let Some(method) = call_method_name(source, node) else {
            return;
        };
        let Some((bad, good)) = map_method(config.get_str("EnforcedStyle", "short"), method) else {
            return;
        };
        report(self, source, node, bad, good, diagnostics, &mut corrections);
    }
}

fn map_method(style: &str, method: &[u8]) -> Option<(&'static str, &'static str)> {
    if style == "short" {
        return match method {
            b"has_key?" => Some(("has_key?", "key?")),
            b"has_value?" => Some(("has_value?", "value?")),
            _ => None,
        };
    }
    match method {
        b"key?" => Some(("key?", "has_key?")),
        b"value?" => Some(("value?", "has_value?")),
        _ => None,
    }
}

fn report(
    cop: &PreferredHashMethods,
    source: &SourceFile,
    node: Node<'_>,
    bad: &str,
    good: &str,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let meth = node.child_by_field_name("method").unwrap_or(node);
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
