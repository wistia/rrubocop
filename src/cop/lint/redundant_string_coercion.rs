//! Lint/RedundantStringCoercion — `.to_s` inside interpolation.

use tree_sitter::Node;

use crate::cop::shared::{call_method_name, call_receiver};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RedundantStringCoercion;

fn to_s_call<'a>(source: &SourceFile, node: Node<'a>) -> Option<Node<'a>> {
    let mut cur = node.walk();
    let named: Vec<_> = node.named_children(&mut cur).collect();
    if named.len() != 1 || named[0].kind() != "call" {
        return None;
    }
    let call = named[0];
    if call_method_name(source, call) != Some(b"to_s") {
        return None;
    }
    call.child_by_field_name("arguments")
        .is_none()
        .then_some(call)
}

fn strip_to_s(source: &SourceFile, call: Node<'_>, recv: Node<'_>) -> Correction {
    let meth = call.child_by_field_name("method").unwrap_or(call);
    let mut start = meth.start_byte();
    if start > recv.end_byte() && source.as_bytes()[start - 1] == b'.' {
        start -= 1;
    }
    Correction {
        start,
        end: meth.end_byte(),
        replacement: String::new(),
        cop_name: "Lint/RedundantStringCoercion",
        cop_index: 0,
        owner: None,
    }
}

fn self_corr(call: Node<'_>) -> Correction {
    Correction {
        start: call.start_byte(),
        end: call.end_byte(),
        replacement: "self".to_string(),
        cop_name: "Lint/RedundantStringCoercion",
        cop_index: 0,
        owner: None,
    }
}

fn maybe_correct(
    source: &SourceFile,
    call: Node<'_>,
    recv: Option<Node<'_>>,
    corrections: Option<&mut Vec<Correction>>,
) -> bool {
    let Some(corr) = corrections else {
        return false;
    };
    if let Some(recv) = recv {
        corr.push(strip_to_s(source, call, recv));
    } else {
        corr.push(self_corr(call));
    }
    true
}

impl Cop for RedundantStringCoercion {
    fn name(&self) -> &'static str {
        "Lint/RedundantStringCoercion"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["interpolation"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(call) = to_s_call(source, node) else {
            return;
        };
        let recv = call_receiver(call);
        let msg = if recv.is_none() {
            "Use `self` instead of `Object#to_s` in interpolation."
        } else {
            "Redundant use of `Object#to_s` in interpolation."
        };
        let (line, col) = source.offset_to_line_col(
            call.child_by_field_name("method")
                .unwrap_or(call)
                .start_byte(),
        );
        let mut diag = self.diagnostic(source, line, col, msg.to_string());
        if maybe_correct(source, call, recv, corrections) {
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}
