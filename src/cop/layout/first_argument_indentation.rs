//! Layout/FirstArgumentIndentation.

mod base;

use tree_sitter::Node;

use crate::cop::layout::first_indent;
use crate::cop::shared::{self, call_method_name};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use base::{base_label, base_text, is_special, prev_code_col, special_col};

pub struct FirstArgumentIndentation;

fn fixed_alignment_disables(config: &CopConfig) -> bool {
    config.get_str("ArgumentAlignmentStyle", "with_first_argument") == "with_fixed_indentation"
        && !config.get_bool("FirstMethodArgumentLineBreakEnabled", false)
}

fn owning_send<'a>(args: Node<'a>) -> Option<Node<'a>> {
    let parent = args.parent()?;
    matches!(parent.kind(), "call" | "command" | "command_call" | "super").then_some(parent)
}

fn setter(name: &[u8]) -> bool {
    name.ends_with(b"=") && !matches!(name, b"==" | b"===" | b"!=" | b"<=" | b">=" | b"=~" | b"!~")
}

fn skip_send(source: &SourceFile, call: Node<'_>) -> bool {
    let Some(name) = call_method_name(source, call) else {
        return false;
    };
    if name == b"[]=" || setter(name) {
        return true;
    }
    call.child_by_field_name("method")
        .is_some_and(|m| m.kind() == "operator")
        && call.child_by_field_name("operator").is_none()
}

fn first_arg<'a>(args: Node<'a>) -> Option<Node<'a>> {
    let mut cur = args.walk();
    args.named_children(&mut cur)
        .find(|n| n.kind() != "comment")
}

fn begins_line(source: &SourceFile, node: Node<'_>) -> bool {
    shared::line_indent(source, node.start_byte()) == shared::node_col(source, node)
}

fn arg_to_check<'a>(
    source: &SourceFile,
    args: Node<'a>,
    config: &CopConfig,
) -> Option<(Node<'a>, Node<'a>)> {
    if fixed_alignment_disables(config) || !first_indent::argument_list_opens_with_paren(args) {
        return None;
    }
    let call = owning_send(args)?;
    if skip_send(source, call) {
        return None;
    }
    let arg = first_arg(args)?;
    if shared::node_line(source, call) == shared::node_line(source, arg)
        || !begins_line(source, arg)
    {
        return None;
    }
    Some((call, arg))
}

fn expected_indent(
    source: &SourceFile,
    call: Node<'_>,
    arg: Node<'_>,
    config: &CopConfig,
) -> (usize, String) {
    let width = config.get_usize("IndentationWidth", 2);
    let special = is_special(
        source,
        call,
        config.get_str(
            "EnforcedStyle",
            "special_for_inner_method_call_in_parentheses",
        ),
    );
    let text = base_text(source, call, arg);
    let base = if special {
        special_col(source, call, &text)
    } else {
        prev_code_col(source, shared::node_line(source, arg))
    };
    (
        base + width,
        format!(
            "Indent the first argument one step more than {}.",
            base_label(&text, special)
        ),
    )
}

fn report(
    cop: &FirstArgumentIndentation,
    source: &SourceFile,
    arg: Node<'_>,
    expected: usize,
    message: String,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(arg.start_byte());
    let mut diag = cop.diagnostic(source, line, col, message);
    if let Some(corr) = corrections
        && let Some(ls) = source.line_start(line)
    {
        corr.push(Correction {
            start: ls,
            end: ls + shared::line_indent(source, arg.start_byte()),
            replacement: " ".repeat(expected),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn check_args(
    cop: &FirstArgumentIndentation,
    source: &SourceFile,
    args: Node<'_>,
    config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let Some((call, arg)) = arg_to_check(source, args, config) else {
        return;
    };
    let (expected, message) = expected_indent(source, call, arg, config);
    if shared::line_indent(source, arg.start_byte()) != expected {
        report(
            cop,
            source,
            arg,
            expected,
            message,
            diagnostics,
            corrections,
        );
    }
}

impl Cop for FirstArgumentIndentation {
    fn name(&self) -> &'static str {
        "Layout/FirstArgumentIndentation"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["argument_list"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        check_args(self, source, node, config, diagnostics, &mut corrections);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        FirstArgumentIndentation,
        "cops/layout/first_argument_indentation"
    );
}
