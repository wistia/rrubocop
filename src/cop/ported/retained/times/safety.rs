use super::*;
mod statement;
pub(super) use statement::statement;

pub(super) fn call(c: &Context<'_>, n: Node<'_>) -> bool {
    !tail(c, n).contains(&b'#')
        && !descendants(n).any(|node| node.kind() == "comment" || node.kind().contains("heredoc"))
}

pub(super) fn removal(c: &Context<'_>, n: Node<'_>) -> bool {
    tail(c, n)
        .iter()
        .all(|byte| byte.is_ascii_whitespace() || *byte == b';')
}

fn tail<'a>(c: &Context<'a>, n: Node<'_>) -> &'a [u8] {
    c.source.as_bytes()[n.end_byte()..]
        .split(|byte| *byte == b'\n')
        .next()
        .unwrap_or_default()
}

pub(super) fn extraction(b: Node<'_>, body: Node<'_>, c: &Context<'_>) -> bool {
    supported_parameters(b)
        && !implicit_parameters(b, body, c)
        && !kids(body)
            .iter()
            .any(|node| matches!(node.kind(), "rescue" | "ensure"))
        && !descendants(body).any(multiline_literal)
        && !unsafe_body(body)
}

fn supported_parameters(b: Node<'_>) -> bool {
    let parameters = field(b, "parameters").map(kids).unwrap_or_default();
    parameters.is_empty() || parameters.len() == 1 && parameters[0].kind() == "identifier"
}

fn implicit_parameters(b: Node<'_>, body: Node<'_>, c: &Context<'_>) -> bool {
    field(b, "parameters").is_none()
        && descendants(body).any(|node| {
            node.kind() == "identifier" && (c.text(node) == "it" || numbered(c.text(node)))
        })
}

fn numbered(name: &str) -> bool {
    name.len() == 2 && name.starts_with('_') && matches!(name.as_bytes()[1], b'1'..=b'9')
}

fn unsafe_body(n: Node<'_>) -> bool {
    if matches!(n.kind(), "break" | "next" | "redo" | "retry") {
        return true;
    }
    if boundary(n.kind()) {
        return false;
    }
    let mut cursor = n.walk();
    n.named_children(&mut cursor).any(unsafe_body)
}

fn boundary(kind: &str) -> bool {
    matches!(
        kind,
        "block"
            | "do_block"
            | "lambda"
            | "method"
            | "singleton_method"
            | "class"
            | "module"
            | "while"
            | "until"
            | "for"
            | "while_modifier"
            | "until_modifier"
    )
}

fn multiline_literal(n: Node<'_>) -> bool {
    n.start_position().row != n.end_position().row
        && (matches!(
            n.kind(),
            "regex" | "delimited_symbol" | "symbol_array" | "subshell"
        ) || n.kind().contains("string"))
}
