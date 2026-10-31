//! Lint/FormatParameterMismatch — format/sprintf/% arg count.

mod scan;

use tree_sitter::Node;

use crate::cop::shared::{argument_nodes, call_method_name, node_bytes, node_text};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct FormatParameterMismatch;

fn string_content(source: &SourceFile, node: Node<'_>) -> Option<String> {
    if node.kind() != "string" {
        return None;
    }
    let mut cur = node.walk();
    let mut out = String::new();
    for c in node.named_children(&mut cur) {
        if c.kind() == "string_content" {
            out.push_str(&node_text(source, c));
        } else if c.kind() == "interpolation" {
            return None;
        }
    }
    Some(out)
}

fn array_len(node: Node<'_>) -> usize {
    let mut cur = node.walk();
    node.named_children(&mut cur).count()
}

fn count_format_args(args: &[Node<'_>]) -> usize {
    if args.is_empty() {
        return 0;
    }
    if args
        .iter()
        .all(|a| a.kind() == "pair" || a.kind() == "hash")
    {
        return 1;
    }
    args.iter().filter(|a| a.kind() != "pair").count()
        + usize::from(args.iter().any(|a| a.kind() == "pair"))
}

fn has_splat_arg(args: &[Node<'_>]) -> bool {
    args.iter().any(|a| a.kind() == "splat_argument")
}

fn report_mismatch(
    cop: &FormatParameterMismatch,
    source: &SourceFile,
    node: Node<'_>,
    method: &str,
    actual: usize,
    expected: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if actual == expected {
        return;
    }
    let (line, col) = source.offset_to_line_col(node.start_byte());
    diagnostics.push(cop.diagnostic(
        source,
        line,
        col,
        format!(
            "Number of arguments ({actual}) to `{method}` doesn't match the number of fields ({expected})."
        ),
    ));
}

fn check_percent(
    cop: &FormatParameterMismatch,
    source: &SourceFile,
    node: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(op) = node.child_by_field_name("operator") else {
        return;
    };
    if node_bytes(source, op) != b"%" {
        return;
    }
    let Some(left) = node.child_by_field_name("left") else {
        return;
    };
    let Some(right) = node.child_by_field_name("right") else {
        return;
    };
    let Some(fmt) = string_content(source, left) else {
        return;
    };
    let Some((expected, named)) = scan::field_count(&fmt) else {
        return;
    };
    if matches!(
        right.kind(),
        "identifier"
            | "call"
            | "element_reference"
            | "instance_variable"
            | "constant"
            | "scope_resolution"
    ) {
        return;
    }
    let actual = if right.kind() == "array" {
        array_len(right)
    } else {
        1
    };
    let expected = if named { 1 } else { expected };
    report_mismatch(cop, source, node, "String#%", actual, expected, diagnostics);
}

fn format_actual_expected(named: bool, field_n: usize, rest: &[Node<'_>]) -> (usize, usize) {
    if named {
        (usize::from(!rest.is_empty()), 1)
    } else {
        (count_format_args(rest), field_n)
    }
}

fn check_format(
    cop: &FormatParameterMismatch,
    source: &SourceFile,
    node: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(meth) = call_method_name(source, node) else {
        return;
    };
    if meth != b"format" && meth != b"sprintf" {
        return;
    }
    let args = argument_nodes(node);
    // RuboCop only checks Kernel-style format/sprintf with a format string plus
    // at least one value arg (`arguments.size > 1`). Skip `obj.format("...")`.
    if args.len() < 2 || has_splat_arg(&args[1..]) {
        return;
    }
    let Some(fmt) = string_content(source, args[0]) else {
        return;
    };
    let Some((field_n, named)) = scan::field_count(&fmt) else {
        return;
    };
    let (actual, expected) = format_actual_expected(named, field_n, &args[1..]);
    report_mismatch(
        cop,
        source,
        node,
        &String::from_utf8_lossy(meth),
        actual,
        expected,
        diagnostics,
    );
}

impl Cop for FormatParameterMismatch {
    fn name(&self) -> &'static str {
        "Lint/FormatParameterMismatch"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "binary"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if node.kind() == "binary" {
            check_percent(self, source, node, diagnostics);
        } else {
            check_format(self, source, node, diagnostics);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        FormatParameterMismatch,
        "cops/lint/format_parameter_mismatch"
    );
}
