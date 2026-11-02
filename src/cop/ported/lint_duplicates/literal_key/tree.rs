use super::*;

pub(super) fn key(c: &Context<'_>, n: Node<'_>, equality: Equality) -> String {
    match n.kind() {
        "array" => array(c, n, equality),
        "symbol_array" | "string_array" => percent_array(c, n),
        "pair" => pair(c, n, equality).unwrap_or_else(|| structural(c, n, equality)),
        "heredoc_beginning" => format!("heredoc:{}", n.start_byte()),
        "rational" | "complex" => raw(c, n),
        _ => structural(c, n, equality),
    }
}
fn array(c: &Context<'_>, n: Node<'_>, equality: Equality) -> String {
    frame(
        "array",
        kids(n).into_iter().map(|n| semantic_key(c, n, equality)),
    )
}
fn percent_array(c: &Context<'_>, n: Node<'_>) -> String {
    kids(n)
        .into_iter()
        .map(|n| strings::key(c, n))
        .collect::<Option<Vec<_>>>()
        .map(|values| frame("array", values))
        .unwrap_or_else(|| raw(c, n))
}
fn pair(c: &Context<'_>, n: Node<'_>, equality: Equality) -> Option<String> {
    Some(frame(
        "pair",
        [
            semantic_key(c, field(n, "key")?, equality),
            semantic_key(c, field(n, "value")?, equality),
        ],
    ))
}
fn structural(c: &Context<'_>, n: Node<'_>, equality: Equality) -> String {
    if n.child_count() == 0 {
        return raw(c, n);
    }
    let mut cursor = n.walk();
    frame(
        n.kind(),
        n.children(&mut cursor)
            .filter(|n| n.kind() != "comment")
            .map(|n| semantic_key(c, n, equality)),
    )
}
fn frame(kind: &str, children: impl IntoIterator<Item = String>) -> String {
    let mut result = format!("{kind}[");
    for child in children {
        result.push_str(&child.len().to_string());
        result.push(':');
        result.push_str(&child);
    }
    result.push(']');
    result
}
fn raw(c: &Context<'_>, n: Node<'_>) -> String {
    format!("{}:{:?}", n.kind(), &c.source.as_bytes()[n.byte_range()])
}
