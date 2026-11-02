use super::super::context::*;
use tree_sitter::Node;
pub(super) fn uri_regexp(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.method(n) != "regexp" || !["URI", "::URI"].contains(&c.receiver(n)) {
        return vec![];
    }
    let preferred = preferred_regexp(c, n);
    let current = c.text(n)[..send_end(n) - n.start_byte()].trim_end();
    let end = n.start_byte() + current.len();
    vec![
        Finding::new(
            method_node(n),
            format!(
                "`{}` is obsolete and should not be used. Instead, use `{preferred}`.",
                current
            ),
        )
        .edit_with(c, n.start_byte(), end, || preferred),
    ]
}
fn preferred_regexp(c: &Context<'_>, n: Node<'_>) -> String {
    let argument = semantic_args(n)
        .first()
        .map(|a| format!("({})", c.text(*a)))
        .unwrap_or_default();
    format!("{}::DEFAULT_PARSER.make_regexp{argument}", c.receiver(n))
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
