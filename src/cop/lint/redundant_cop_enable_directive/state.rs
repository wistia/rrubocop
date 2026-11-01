use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Default)]
pub(super) struct Disabled(HashMap<String, usize>);

impl Disabled {
    pub(super) fn new(names: std::collections::HashSet<String>) -> Self {
        let mut disabled = Self::default();
        for name in names {
            disabled.disable(&name);
        }
        disabled
    }
    pub(super) fn disable(&mut self, selector: &str) {
        for name in selected(selector) {
            *self.0.entry(name).or_default() += 1;
        }
    }
    pub(super) fn disable_comment(&mut self, line: &str) {
        if !line.trim_start().starts_with('#') {
            return;
        }
        let rest = line
            .split("# rubocop:disable")
            .nth(1)
            .or_else(|| line.split("# rubocop:todo").nth(1));
        if let Some(rest) = rest {
            for name in super::directive_names(rest) {
                self.disable(&name);
            }
        }
    }
    pub(super) fn enable(&mut self, selector: &str) -> bool {
        if selector.eq_ignore_ascii_case("all") {
            return self.enable_all();
        }
        let mut matched = true;
        for name in selected(selector) {
            if let Some(count) = self.0.get_mut(&name).filter(|count| **count > 0) {
                *count -= 1;
            } else {
                matched = false;
            }
        }
        matched
    }
    fn enable_all(&mut self) -> bool {
        let mut matched = false;
        for count in self.0.values_mut().filter(|count| **count > 0) {
            *count -= 1;
            matched = true;
        }
        matched
    }
}
fn selected(selector: &str) -> Vec<String> {
    let selector = crate::config::cop_names::qualified(selector).unwrap_or_else(|| selector.into());
    if selector.contains('/') {
        return vec![selector.to_ascii_lowercase()];
    }
    let names: Vec<_> = names()
        .iter()
        .filter(|name| matches_selector(&selector, name))
        .map(|name| name.to_ascii_lowercase())
        .collect();
    if names.is_empty() {
        vec![selector.to_ascii_lowercase()]
    } else {
        names
    }
}
fn matches_selector(selector: &str, name: &str) -> bool {
    if name == "Lint/Syntax" || name == "Lint/RedundantCopDisableDirective" {
        return false;
    }
    selector.eq_ignore_ascii_case("all")
        || name
            .split_once('/')
            .is_some_and(|(department, _)| selector.eq_ignore_ascii_case(department))
}
fn names() -> &'static Vec<&'static str> {
    static NAMES: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
        let mut names = crate::cop::CopRegistry::default_registry().names();
        names.extend(include_str!("../../../resources/cop_department_names.txt").lines());
        names.sort_unstable();
        names.dedup();
        names
    });
    &NAMES
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_names_keep_orphan_and_matching_disable_behavior() {
        let mut disabled = Disabled::default();
        assert!(!disabled.enable("MadeUp"));
        disabled.disable("MadeUp");
        assert!(disabled.enable("MadeUp"));
        assert!(!disabled.enable("MadeUp"));
    }
}
