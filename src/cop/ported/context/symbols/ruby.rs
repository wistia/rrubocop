use super::*;
mod heredoc;
pub(in crate::cop::ported::context) use heredoc::content_span as heredoc_span;

pub(in crate::cop::ported::context) fn string<'a>(
    c: &Context<'a>,
    n: Node<'_>,
) -> Option<Cow<'a, str>> {
    match n.kind() {
        "string" => super::string(c, n),
        "character" => character(c.text(n)),
        "heredoc_beginning" => heredoc::value(c, n),
        _ => None,
    }
}

fn character(text: &str) -> Option<Cow<'_, str>> {
    let value = text.strip_prefix('?')?;
    if value.contains('\\') {
        decode(value, true, b'"').map(Cow::Owned)
    } else {
        Some(Cow::Borrowed(value))
    }
}

#[cfg(test)]
mod tests;

pub(in crate::cop::ported::context) fn parts(c: &Context<'_>, n: Node<'_>) -> Option<Vec<String>> {
    match n.kind() {
        "string" => super::string_parts(c, n),
        "character" => character(c.text(n)).map(|value| vec![value.into_owned()]),
        "heredoc_beginning" => heredoc::parts(c, n),
        _ => None,
    }
}
