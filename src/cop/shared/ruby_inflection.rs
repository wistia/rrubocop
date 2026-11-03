//! ActiveSupport's default, empty-acronym underscore and foreign-key algorithms.
use regex::Regex;
use std::sync::LazyLock;

pub fn underscore(name: &str) -> String {
    if !name.bytes().any(|b| b.is_ascii_uppercase() || b == b'-') && !name.contains("::") {
        return name.into();
    }
    static ACRONYM_BOUNDARY: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"([A-Z0-9]+)([A-Z][a-z])").unwrap());
    static WORD_BOUNDARY: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"([a-z0-9])([A-Z])").unwrap());
    let normalized = name.replace("::", "/");
    let normalized = ACRONYM_BOUNDARY.replace_all(&normalized, "${1}_${2}");
    WORD_BOUNDARY
        .replace_all(&normalized, "${1}_${2}")
        .replace('-', "_")
        .chars()
        .flat_map(char::to_lowercase)
        .collect()
}

pub fn foreign_key(name: &str) -> String {
    format!(
        "{}_id",
        underscore(name.rsplit("::").next().unwrap_or(name))
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn inflections_match_pinned_active_support() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/framework-inflections.json"
        )))
        .unwrap();
        for row in fixture["cases"].as_array().unwrap() {
            let source = row["source"].as_str().unwrap();
            assert_eq!(
                super::underscore(source),
                row["underscore"].as_str().unwrap(),
                "{source:?}"
            );
            assert_eq!(
                super::foreign_key(source),
                row["foreign_key"].as_str().unwrap(),
                "{source:?}"
            );
        }
    }
}
