use crate::parse::source::SourceFile;
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct PatternSets(Mutex<HashMap<Vec<String>, Arc<Vec<Regex>>>>);

pub(crate) fn compiled_patterns(source: &SourceFile, patterns: Vec<String>) -> Arc<Vec<Regex>> {
    if patterns.is_empty() {
        static EMPTY: std::sync::LazyLock<Arc<Vec<Regex>>> = std::sync::LazyLock::new(Arc::default);
        return EMPTY.clone();
    }
    let sets = source
        .operation()
        .cached(Path::new(""), PatternSets::default);
    let mut sets = sets.0.lock().unwrap();
    sets.entry(patterns)
        .or_insert_with_key(|patterns| {
            Arc::new(
                patterns
                    .iter()
                    .filter_map(|pattern| Regex::new(pattern).ok())
                    .collect(),
            )
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pattern_sets_share_only_within_operation_and_complete_pattern_identity() {
        let operation = Arc::default();
        let first = SourceFile::from_bytes("a.rb", vec![]).with_operation(Arc::clone(&operation));
        let second = SourceFile::from_bytes("b.rb", vec![]).with_operation(operation);
        let patterns = vec!["^allow".into(), "[".into(), "deny$".into()];
        let compiled = compiled_patterns(&first, patterns.clone());
        assert_eq!(compiled.len(), 2);
        assert!(compiled[0].is_match("allowed"));
        assert!(compiled[1].is_match("deny"));
        assert!(Arc::ptr_eq(
            &compiled,
            &compiled_patterns(&second, patterns.clone())
        ));
        assert!(!Arc::ptr_eq(
            &compiled,
            &compiled_patterns(&first, vec!["deny$".into()])
        ));
        let fresh = SourceFile::from_bytes("a.rb", vec![]);
        assert!(!Arc::ptr_eq(
            &compiled,
            &compiled_patterns(&fresh, patterns)
        ));
    }
}
