//! Style/ClassCheck — prefer is_a? or kind_of?.

use tree_sitter::Node;

use crate::cop::shared::call_method_name;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ClassCheck;

impl Cop for ClassCheck {
    fn name(&self) -> &'static str {
        "Style/ClassCheck"
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
        let Some((prefer, current)) =
            preferred_pair(config.get_str("EnforcedStyle", "is_a?"), method)
        else {
            return;
        };
        report(
            self,
            source,
            node,
            prefer,
            current,
            diagnostics,
            &mut corrections,
        );
    }
}

fn preferred_pair(enforced: &str, method: &[u8]) -> Option<(&'static str, &'static str)> {
    let (prefer, current) = if enforced == "is_a?" {
        ("is_a?", "kind_of?")
    } else {
        ("kind_of?", "is_a?")
    };
    if method != current.as_bytes() {
        return None;
    }
    Some((prefer, current))
}

fn report(
    cop: &ClassCheck,
    source: &SourceFile,
    node: Node<'_>,
    prefer: &str,
    current: &str,
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
        format!("Prefer `Object#{prefer}` over `Object#{current}`."),
    );
    if let Some(corr) = corrections.as_mut() {
        corr.push(Correction {
            start: meth.start_byte(),
            end: meth.end_byte(),
            replacement: prefer.to_string(),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}
