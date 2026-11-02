use super::super::context::*;
use tree_sitter::Node;
pub(super) fn uri_regexp(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.method(n) != "regexp" || !["URI", "::URI"].contains(&c.receiver(n)) {
        return vec![];
    }
    vec![regexp_finding(c, n)]
}
fn regexp_finding(c: &Context<'_>, n: Node<'_>) -> Finding {
    let end = Finding::send(n, "").end;
    let prefix = format!("{}::DEFAULT_PARSER.make_regexp", c.receiver(n));
    let argument = semantic_args(n).first().copied();
    let finding = regexp_diagnostic(c, n, end, &prefix, argument);
    match argument {
        Some(argument) => finding
            .edit_with(c, n.start_byte(), argument.start_byte(), || {
                format!("{prefix}(")
            })
            .also_edit_with(c, argument.end_byte(), end, || ")"),
        None => finding.edit_with(c, n.start_byte(), end, || prefix),
    }
}
fn regexp_diagnostic(
    c: &Context<'_>,
    n: Node<'_>,
    end: usize,
    prefix: &str,
    argument: Option<Node<'_>>,
) -> Finding {
    let current = String::from_utf8_lossy(&c.source.as_bytes()[n.start_byte()..end]);
    let preferred = preferred_regexp(c, prefix, argument);
    Finding::new(
        method_node(n),
        format!("`{current}` is obsolete and should not be used. Instead, use `{preferred}`."),
    )
}
fn preferred_regexp(c: &Context<'_>, prefix: &str, argument: Option<Node<'_>>) -> String {
    argument.map_or_else(
        || prefix.into(),
        |argument| {
            format!(
                "{prefix}({})",
                String::from_utf8_lossy(&c.source.as_bytes()[argument.byte_range()])
            )
        },
    )
}
pub(super) fn uri_escape(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if !["URI", "::URI"].contains(&c.receiver(n)) {
        return vec![];
    }
    let replacement = match c.method(n) {
        "escape" | "encode" => {
            "`CGI.escape`, `URI.encode_www_form` or `URI.encode_www_form_component`"
        }
        "unescape" | "decode" => {
            "`CGI.unescape`, `URI.decode_www_form` or `URI.decode_www_form_component`"
        }
        _ => return vec![],
    };
    vec![Finding::new(
        n,
        format!(
            "`{}.{}` method is obsolete and should not be used. Instead, use {replacement} depending on your specific use case.",
            c.receiver(n),
            c.method(n)
        ),
    )]
}

#[cfg(test)]
mod byte_tests;
