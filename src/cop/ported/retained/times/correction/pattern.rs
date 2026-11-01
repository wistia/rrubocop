use super::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub(super) struct ParameterPattern(pub regex::Regex);
#[derive(Default)]
struct Patterns(Mutex<HashMap<String, Arc<ParameterPattern>>>);

pub(super) fn parameter_pattern(c: &Context<'_>, parameter: &str) -> Arc<ParameterPattern> {
    c.source.cached(0, Patterns::default).get(parameter)
}

impl Patterns {
    fn get(&self, parameter: &str) -> Arc<ParameterPattern> {
        let mut patterns = self.0.lock().unwrap();
        if let Some(pattern) = patterns.get(parameter) {
            return pattern.clone();
        }
        let pattern = Arc::new(ParameterPattern(
            regex::Regex::new(&format!(r"\b{}\b", regex::escape(parameter))).unwrap(),
        ));
        patterns.insert(parameter.into(), pattern.clone());
        pattern
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equal_parameter_names_share_compiled_patterns() {
        let cache = Patterns::default();
        let first = String::from("item");
        let second = String::from("item");
        assert!(Arc::ptr_eq(&cache.get(&first), &cache.get(&second)));
        assert!(!Arc::ptr_eq(&cache.get("item"), &cache.get("other")));
    }
}
