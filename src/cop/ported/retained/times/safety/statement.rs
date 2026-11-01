use super::*;

pub(in crate::cop::ported::retained::times) fn statement(n: Node<'_>) -> bool {
    let Some(parent) = n.parent() else {
        return false;
    };
    terminal(parent, n) || transports(parent, n) && statement(parent)
}

fn terminal(parent: Node<'_>, n: Node<'_>) -> bool {
    match parent.kind() {
        "program" | "do" | "ensure" => true,
        "method" | "singleton_method" | "block" | "do_block" | "lambda" => {
            field(parent, "body") == Some(n)
        }
        _ => false,
    }
}

fn transports(parent: Node<'_>, n: Node<'_>) -> bool {
    match parent.kind() {
        "body_statement" | "block_body" | "then" | "else" | "begin" => true,
        "class" | "module" => field(parent, "body") == Some(n),
        "if" | "unless" | "elsif" => field(parent, "condition") != Some(n),
        "case" => field(parent, "value") != Some(n),
        "when" | "rescue" => n.kind() == "then",
        _ => false,
    }
}
