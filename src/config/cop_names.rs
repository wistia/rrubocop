//! Qualify unambiguous legacy cop keys using the installed Rust registry.
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::OnceLock;

pub(super) fn qualified(name: &str) -> Option<Cow<'_, str>> {
    if name.contains('/') {
        return Some(Cow::Borrowed(name));
    }
    if name.contains("::") {
        return Some(Cow::Owned(name.replace("::", "/")));
    }
    short_names()
        .get(name)
        .copied()
        .flatten()
        .map(Cow::Borrowed)
}
fn short_names() -> &'static HashMap<&'static str, Option<&'static str>> {
    static NAMES: OnceLock<HashMap<&'static str, Option<&'static str>>> = OnceLock::new();
    NAMES.get_or_init(|| {
        let mut names = HashMap::new();
        for name in crate::cop::CopRegistry::default_registry().names() {
            if let Some((_, short)) = name.split_once('/') {
                names
                    .entry(short)
                    .and_modify(|found| *found = None)
                    .or_insert(Some(name));
            }
        }
        names
    })
}
