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
