//! Heuristic matchers for breadth-first Style cops.
use crate::cop::CopConfig;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub fn matches_if_with_semicolon(source: &SourceFile, node: Node<'_>, _config: &CopConfig) -> bool {
    if node.kind() != "if" && node.kind() != "unless" {
        return false;
    }
    // `if cond; body` — then child starts with ;
    let mut cur = node.walk();
    let Some(then_n) = node.children(&mut cur).find(|ch| ch.kind() == "then") else {
        return false;
    };
    let b = &source.as_bytes()[then_n.start_byte()..then_n.end_byte()];
    b.starts_with(b";")
}

pub fn matches_one_line_conditional(
    _source: &SourceFile,
    node: Node<'_>,
    _config: &CopConfig,
) -> bool {
    if !matches!(node.kind(), "if" | "unless") {
        return false;
    }
    if node.start_position().row != node.end_position().row {
        return false;
    }
    let mut cur = node.walk();
    node.children(&mut cur).any(|ch| ch.kind() == "else")
}

pub fn matches_stabby_lambda_parentheses(
    source: &SourceFile,
    node: Node<'_>,
    config: &CopConfig,
) -> bool {
    if node.kind() != "lambda" {
        return false;
    }
    // RuboCop only applies when the lambda has arguments.
    let Some(params) = node.child_by_field_name("parameters") else {
        return false;
    };
    let style = config.get_str("EnforcedStyle", "require_parentheses");
    let bytes = &source.as_bytes()[params.start_byte()..params.end_byte()];
    let has_parens = bytes.starts_with(b"(");
    match style {
        "require_parentheses" => !has_parens,
        "require_no_parentheses" => has_parens,
        _ => false,
    }
}
