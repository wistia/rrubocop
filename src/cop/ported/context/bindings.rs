use super::*;

pub(in crate::cop::ported) fn local_identifier(c: &Context<'_>, node: Node<'_>) -> bool {
    let Ok(tree) = crate::parse::parse_ruby(c.source) else {
        return false;
    };
    crate::model::build(c.source.as_bytes(), tree)
        .scopes
        .iter()
        .any(|scope| {
            scope.entries.values().any(|entry| {
                entry.intro_byte == node.start_byte()
                    || entry
                        .writes
                        .iter()
                        .any(|write| write.byte == node.start_byte())
                    || entry
                        .reads
                        .iter()
                        .any(|read| read.byte == node.start_byte())
            })
        })
}
