use super::*;
use crate::model::{LocalIdentifierScopes, LocalIdentifiers};

pub(in crate::cop::ported) fn local_identifier(c: &Context<'_>, node: Node<'_>) -> bool {
    c.source
        .cached::<LocalIdentifiers>(0, || {
            let scopes = c.source.cached(0, || {
                let scopes = crate::parse::parse_ruby(c.source)
                    .map(|tree| crate::model::build(c.source.as_bytes(), tree).scopes)
                    .unwrap_or_default();
                LocalIdentifierScopes::new(scopes)
            });
            scopes.index()
        })
        .contains(node.start_byte())
}
