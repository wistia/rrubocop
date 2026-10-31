//! Naming/ConstantName — constants must be SCREAMING_SNAKE_CASE.

use tree_sitter::Node;

use crate::cop::shared::{
    call_method_name, call_receiver, is_const_named, is_screaming_snake_case, node_bytes,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ConstantName;

impl Cop for ConstantName {
    fn name(&self) -> &'static str {
        "Naming/ConstantName"
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["assignment", "operator_assignment"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let Some(const_node) = const_lhs(node) else {
            return;
        };
        if is_screaming_snake_case(node_bytes(source, const_node)) {
            return;
        }
        if allowed_assignment(
            source,
            node.child_by_field_name("right")
                .or_else(|| node.child_by_field_name("value")),
        ) {
            return;
        }
        let (line, column) = source.offset_to_line_col(const_node.start_byte());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use SCREAMING_SNAKE_CASE for constants. (https://rubystyle.guide#screaming-snake-case-constants)"
                .to_string(),
        ));
    }
}

fn const_lhs(node: Node<'_>) -> Option<Node<'_>> {
    let left = node.child_by_field_name("left")?;
    let const_node = match left.kind() {
        "constant" => left,
        "scope_resolution" => left.child_by_field_name("name").unwrap_or(left),
        _ => return None,
    };
    (const_node.kind() == "constant").then_some(const_node)
}

fn allowed_assignment(source: &SourceFile, value: Option<Node<'_>>) -> bool {
    let Some(value) = value else {
        return true;
    };
    match value.kind() {
        "constant" | "scope_resolution" | "block" | "do_block" | "identifier" => true,
        "call" | "command" => {
            class_or_struct_new(source, value) || allowed_method_call_on_rhs(value)
        }
        "parenthesized_statements" => {
            let mut cur = value.walk();
            let inner: Vec<_> = value.named_children(&mut cur).collect();
            inner.len() == 1 && allowed_assignment(source, Some(inner[0]))
        }
        "if" | "unless" | "case" => contains_constant_branch(value),
        _ => false,
    }
}

fn class_or_struct_new(source: &SourceFile, node: Node<'_>) -> bool {
    call_method_name(source, node) == Some(b"new")
        && call_receiver(node).is_some_and(|r| {
            is_const_named(source, r, b"Class") || is_const_named(source, r, b"Struct")
        })
}

fn allowed_method_call_on_rhs(node: Node<'_>) -> bool {
    // RuboCop: send with no receiver, or non-literal receiver — cannot know type.
    let Some(recv) = call_receiver(node) else {
        return true;
    };
    !literal_receiver(recv)
}

fn literal_receiver(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "integer"
            | "float"
            | "complex"
            | "rational"
            | "string"
            | "string_content"
            | "simple_symbol"
            | "symbol"
            | "array"
            | "hash"
            | "regex"
            | "true"
            | "false"
            | "nil"
            | "heredoc_beginning"
    ) || (node.kind() == "parenthesized_statements" && {
        let mut cur = node.walk();
        let kids: Vec<_> = node.named_children(&mut cur).collect();
        kids.len() == 1 && literal_receiver(kids[0])
    })
}

fn contains_constant_branch(node: Node<'_>) -> bool {
    let mut cur = node.walk();
    node.named_children(&mut cur).any(|n| match n.kind() {
        "constant" | "scope_resolution" => true,
        "then" | "else" | "elsif" | "when" | "in" | "body_statement" => contains_constant_branch(n),
        _ => false,
    })
}
