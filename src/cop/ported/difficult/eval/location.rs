use super::super::super::context::*;
use tree_sitter::Node;
mod heredoc;
mod number;
pub(super) fn first_line(c: &Context<'_>, code: Node<'_>) -> usize {
    heredoc::first_line(c, code)
}
pub(super) fn difference(c: &Context<'_>, code: Node<'_>, line: Node<'_>) -> isize {
    first_line(c, code) as isize - line.start_position().row as isize - 1
}
pub(super) fn expected(diff: isize) -> String {
    match diff.cmp(&0) {
        std::cmp::Ordering::Equal => "__LINE__".into(),
        std::cmp::Ordering::Greater => format!("__LINE__ + {diff}"),
        std::cmp::Ordering::Less => format!("__LINE__ - {}", diff.abs()),
    }
}
pub(super) fn variable_line(c: &Context<'_>, line: Node<'_>) -> bool {
    match line.kind() {
        "identifier" => c.text(line) != "__LINE__",
        "instance_variable" | "class_variable" | "global_variable" => true,
        "call" => c.method(line) != "+",
        "binary" => operator(c, line) != "+",
        _ => false,
    }
}
pub(super) fn correct(c: &Context<'_>, line: Node<'_>, diff: isize) -> bool {
    if diff == 0 {
        return c.text(line) == "__LINE__";
    }
    diff > 0 && line_offset(c, line).is_some_and(|value| value == diff)
}
fn line_offset(c: &Context<'_>, n: Node<'_>) -> Option<isize> {
    if n.kind() != "binary" || operator(c, n) != "+" {
        return None;
    }
    let (left, right) = (field(n, "left")?, field(n, "right")?);
    let number = if c.text(left) == "__LINE__" {
        right
    } else if c.text(right) == "__LINE__" {
        left
    } else {
        return None;
    };
    number::parse(c.text(number))
}
pub(super) fn replaceable(c: &Context<'_>, n: Node<'_>) -> bool {
    match n.kind() {
        "integer" | "float" | "simple_symbol" | "nil" | "true" | "false" => true,
        "identifier" => matches!(c.text(n), "__FILE__" | "__LINE__") || local_identifier(c, n),
        "string" => !descendants(n).iter().any(|n| n.kind() == "interpolation"),
        "binary" => line_offset(c, n).is_some(),
        "unary" => signed_number(c, n),
        _ => false,
    }
}
fn signed_number(c: &Context<'_>, n: Node<'_>) -> bool {
    matches!(operator(c, n), "+" | "-")
        && kids(n)
            .iter()
            .all(|n| matches!(n.kind(), "integer" | "float"))
}
