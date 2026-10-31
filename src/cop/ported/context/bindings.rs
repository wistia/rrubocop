use super::*;
use crate::model::LocalIdentifiers;

pub(in crate::cop::ported) fn local_identifier(c: &Context<'_>, node: Node<'_>) -> bool {
    c.source
        .cached(0, || {
            crate::parse::parse_ruby(c.source)
                .map(|tree| {
                    LocalIdentifiers::from_model(&crate::model::build(c.source.as_bytes(), tree))
                })
                .unwrap_or_default()
        })
        .contains(node.start_byte())
}
