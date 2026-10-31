use super::super::super::context::*;
use tree_sitter::Node;

pub(super) fn not_a_call(c: &Context<'_>, key: Node<'_>) -> bool {
    match key.kind() {
        "identifier" => local_identifier(c, key),
        "unary" => numeric_unary(c, key),
        "call" | "command" | "command_call" | "binary" | "element_reference" => false,
        _ => true,
    }
}

fn numeric_unary(c: &Context<'_>, key: Node<'_>) -> bool {
    ["-", "+"].contains(&operator(c, key))
        && field(key, "operand")
            .is_some_and(|operand| matches!(operand.kind(), "integer" | "float"))
}
