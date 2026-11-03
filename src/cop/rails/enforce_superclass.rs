use crate::cop::shared::ruby_arguments::arguments;
use crate::cop::shared::{
    call_method_name, call_receiver, for_each_descendant, named_kids, node_text, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
fn constant(source: &SourceFile, node: Node<'_>, path: &str) -> bool {
    crate::cop::ported::constant_name(source, node).as_deref() == Some(path)
}
fn terminal_constant(source: &SourceFile, node: Node<'_>) -> Option<String> {
    if node.kind() == "constant" {
        Some(node_text(source, node))
    } else if node.kind() == "scope_resolution" {
        node.child_by_field_name("name")
            .map(|n| node_text(source, n))
    } else {
        None
    }
}
fn root_assignment(source: &SourceFile, node: Node<'_>, prefer: &str) -> bool {
    node.kind() == "assignment"
        && node
            .child_by_field_name("left")
            .is_some_and(|left| constant(source, left, prefer))
}
pub fn check_superclass(
    cop: &dyn Cop,
    source: &SourceFile,
    node: Node<'_>,
    base: &str,
    prefer: &str,
    message: &str,
    config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    mut corrections: Option<&mut Vec<Correction>>,
) {
    if prefer != "ApplicationController"
        && (config.get_f64("TargetRailsVersion", 8.1) < 5.0
            || !config.get_bool("__RailtiesInLockfile", true)
            || config.get_f64(
                "__RailtiesVersion",
                config.get_f64("TargetRailsVersion", 8.1),
            ) < 5.0)
    {
        return;
    }
    let superclass = if node.kind() == "class" {
        if node
            .child_by_field_name("name")
            .and_then(|n| terminal_constant(source, n))
            .as_deref()
            == Some(prefer)
        {
            return;
        }
        let Some(superclass) = node
            .child_by_field_name("superclass")
            .and_then(|n| named_kids(n).into_iter().find(|n| n.kind() != "comment"))
        else {
            return;
        };
        superclass
    } else {
        if crate::cop::shared::ruby_expression::regular_send_receiver(source, node).is_none()
            || crate::cop::shared::is_setter_call(node)
            || call_method_name(source, node) != Some(b"new")
            || !call_receiver(node).is_some_and(|n| constant(source, n, "Class"))
        {
            return;
        }
        let args = arguments(node);
        if args.len() != 1 {
            return;
        }
        let Some(superclass) = args[0].node() else {
            return;
        };
        let mut parent = node.parent();
        if parent.is_some_and(|p| p.kind() == "argument_list") {
            parent = parent.and_then(|p| p.parent());
        }
        if parent.is_some_and(|p| root_assignment(source, p, prefer)) {
            return;
        }
        superclass
    };
    if !constant(source, superclass, base) {
        return;
    }
    let (line, col) = source.offset_to_line_col(superclass.start_byte());
    let mut d = cop.diagnostic(source, line, col, message.to_owned());
    d.highlight_length =
        super::blank::highlight_length(source, superclass.start_byte(), superclass.end_byte());
    if let Some(s) = config.severity {
        d.severity = s;
    }
    let mut comment = false;
    for_each_descendant(superclass, |n| comment |= n.kind() == "comment");
    d.correctable = !config.autocorrect_disabled() && !comment;
    if d.correctable {
        d.corrected = push_replace(
            &mut corrections,
            superclass.start_byte(),
            superclass.end_byte(),
            prefer,
            cop.name(),
        );
    }
    diagnostics.push(d);
}
