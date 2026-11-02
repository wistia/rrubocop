use super::*;
use crate::model::{LocalIdentifierScopes, LocalIdentifiers};

fn index(c: &Context<'_>) -> std::sync::Arc<LocalIdentifiers> {
    c.source.cached::<LocalIdentifiers>(0, || {
        let scopes = c.source.cached(0, || {
            let scopes = crate::parse::parse_ruby(c.source)
                .map(|tree| crate::model::build(c.source.as_bytes(), tree).scopes)
                .unwrap_or_default();
            LocalIdentifierScopes::new(scopes)
        });
        scopes.index()
    })
}

pub(in crate::cop::ported) fn local_identifier(c: &Context<'_>, node: Node<'_>) -> bool {
    index(c).contains(node.start_byte())
}

pub(in crate::cop::ported) fn local_binding(c: &Context<'_>, node: Node<'_>) -> Option<usize> {
    index(c).binding(node.start_byte())
}

pub(in crate::cop::ported) fn numbered_bindings(
    c: &Context<'_>,
    node: Node<'_>,
) -> Option<Vec<usize>> {
    if node.child_by_field_name("parameters").is_some() {
        return None;
    }
    let block = if node.kind() == "lambda" {
        node.child_by_field_name("body")?
    } else {
        node
    };
    if !matches!(block.kind(), "block" | "do_block") {
        return None;
    }
    index(c).numbered_bindings(block.start_byte())
}
