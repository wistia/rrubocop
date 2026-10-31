use super::FileModel;
use std::collections::HashSet;

#[derive(Default)]
pub(crate) struct LocalIdentifiers(HashSet<usize>);

impl LocalIdentifiers {
    pub(crate) fn from_model(model: &FileModel<'_>) -> Self {
        let mut bytes = HashSet::new();
        for scope in &model.scopes {
            for entry in scope.entries.values() {
                bytes.insert(entry.intro_byte);
                bytes.extend(entry.writes.iter().map(|write| write.byte));
                bytes.extend(entry.reads.iter().map(|read| read.byte));
            }
        }
        Self(bytes)
    }

    pub(crate) fn contains(&self, byte: usize) -> bool {
        self.0.contains(&byte)
    }
}
