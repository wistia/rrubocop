use super::ScopeData;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

pub(crate) struct LocalIdentifierScopes(Mutex<Option<Arc<Vec<ScopeData>>>>);

impl LocalIdentifierScopes {
    pub(crate) fn new(scopes: Arc<Vec<ScopeData>>) -> Self {
        Self(Mutex::new(Some(scopes)))
    }

    pub(crate) fn index(&self) -> LocalIdentifiers {
        let scopes = self
            .0
            .lock()
            .unwrap()
            .take()
            .expect("identifier scopes indexed once");
        LocalIdentifiers::from_scopes(&scopes)
    }

    #[cfg(test)]
    pub(crate) fn shares(&self, scopes: &Arc<Vec<ScopeData>>) -> bool {
        self.0
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|seed| Arc::ptr_eq(seed, scopes))
    }

    #[cfg(test)]
    pub(crate) fn released(&self) -> bool {
        self.0.lock().unwrap().is_none()
    }
}

#[derive(Default)]
pub(crate) struct LocalIdentifiers(HashSet<usize>);

impl LocalIdentifiers {
    pub(crate) fn from_scopes(scopes: &[ScopeData]) -> Self {
        let mut bytes = HashSet::new();
        for scope in scopes {
            for entry in scope.entries.values() {
                bytes.insert(entry.intro_byte);
                bytes.extend(entry.writes.iter().map(|write| write.byte));
                bytes.extend(
                    entry
                        .reads
                        .iter()
                        .filter(|read| !read.synthetic)
                        .map(|read| read.byte),
                );
            }
        }
        Self(bytes)
    }

    pub(crate) fn contains(&self, byte: usize) -> bool {
        self.0.contains(&byte)
    }
}
