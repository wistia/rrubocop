use super::*;
mod encoding;

pub(super) fn key(c: &Context<'_>, n: Node<'_>) -> Option<String> {
    let kind = kind(c, n)?;
    if !encoding::supported_raw(c, n) {
        return None;
    }
    let (bytes, unicode) = c.duplicate_literal_bytes(n)?;
    let encoding = encoding::name(c, &bytes, unicode);
    Some(format!("{kind}:{encoding}:{bytes:?}"))
}
fn kind(c: &Context<'_>, n: Node<'_>) -> Option<&'static str> {
    match n.kind() {
        "simple_symbol" | "delimited_symbol" | "hash_key_symbol" | "bare_symbol" => Some("symbol"),
        "string" if symbol_label(c, n) => Some("symbol"),
        "string" | "character" | "bare_string" => Some("string"),
        _ => None,
    }
}
fn symbol_label(c: &Context<'_>, n: Node<'_>) -> bool {
    c.source.as_bytes().get(n.end_byte()) == Some(&b':')
        && n.parent()
            .is_some_and(|p| p.kind() == "pair" && field(p, "key") == Some(n))
}
