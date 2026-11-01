use super::*;

pub(super) fn write(c: &Context<'_>, call: Node<'_>, target: Node<'_>, content: Node<'_>) -> bool {
    args(call).len() == 2
        && !descendants(target)
            .any(|node| node.kind() == "comment" || node.kind().contains("heredoc"))
        && stable_content(c, call, content)
}
fn stable_content(c: &Context<'_>, call: Node<'_>, content: Node<'_>) -> bool {
    match content.kind() {
        "string" => c.static_string(content).is_some(),
        "simple_symbol" | "delimited_symbol" => c.static_symbol(content).is_some(),
        "integer" | "float" | "true" | "false" | "nil" => true,
        "identifier" => {
            local_identifier(c, content)
                && !removed_parameter(c, call, content)
                && args(call)
                    .first()
                    .is_some_and(|path| c.static_string(*path).is_some())
        }
        _ => false,
    }
}
fn removed_parameter(c: &Context<'_>, call: Node<'_>, content: Node<'_>) -> bool {
    block(call)
        .and_then(|block| field(block, "parameters"))
        .is_some_and(|parameters| {
            kids(parameters)
                .iter()
                .any(|parameter| c.text(*parameter) == c.text(content))
        })
}
