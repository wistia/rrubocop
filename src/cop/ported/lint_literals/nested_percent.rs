use super::super::context::*;
use tree_sitter::Node;
pub(super) fn nested_percent(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if kids(n).iter().any(|ch| nested(c.text(*ch).as_bytes())) {
        vec![Finding::new(
            n,
            "Within percent literals, nested percent literals do not function and may be unwanted in the result.",
        )]
    } else {
        vec![]
    }
}

fn nested(b: &[u8]) -> bool {
    b.len() > 2 && b[0] == b'%' && b"wWiIqQrxs".contains(&b[1]) && !b[2].is_ascii_alphanumeric()
}
