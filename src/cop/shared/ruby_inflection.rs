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

pub fn pluralize(word: &str) -> String {
    static UNCOUNTABLE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\b(?:equipment|information|rice|money|species|series|fish|sheep|jeans|police)(?:\n)?\z").unwrap()
    });
    static RULES: LazyLock<Vec<(Regex, String)>> = LazyLock::new(|| {
        let mut rules = Vec::new();
        for (singular, plural) in [
            ("person", "people"),
            ("man", "men"),
            ("child", "children"),
            ("sex", "sexes"),
            ("move", "moves"),
            ("zombie", "zombies"),
        ]
        .into_iter()
        .rev()
        {
            for suffix in [plural, singular] {
                rules.push((
                    Regex::new(&format!("(?mi)({}){}$", &suffix[..1], &suffix[1..])).unwrap(),
                    format!("${{1}}{}", &plural[1..]),
                ));
            }
        }
        let defaults = [
            (r"$", "s"),
            (r"s$", "s"),
            (r"^(ax|test)is$", "${1}es"),
            (r"(octop|vir)us$", "${1}i"),
            (r"(octop|vir)i$", "${1}i"),
            (r"(alias|status)$", "${1}es"),
            (r"(bu)s$", "${1}ses"),
            (r"(buffal|tomat)o$", "${1}oes"),
            (r"([ti])um$", "${1}a"),
            (r"([ti])a$", "${1}a"),
            (r"sis$", "ses"),
            (r"(?:([^f])fe|([lr])f)$", "${1}${2}ves"),
            (r"(hive)$", "${1}s"),
            (r"([^aeiouy]|qu)y$", "${1}ies"),
            (r"(x|ch|ss|sh)$", "${1}es"),
            (r"(matr|vert|ind)(?:ix|ex)$", "${1}ices"),
            (r"^(m|l)ouse$", "${1}ice"),
            (r"^(m|l)ice$", "${1}ice"),
            (r"^(ox)$", "${1}en"),
            (r"^(oxen)$", "${1}"),
            (r"(quiz)$", "${1}zes"),
        ];
        rules.extend(defaults.into_iter().rev().map(|(pattern, replacement)| {
            (
                Regex::new(&format!("(?mi){pattern}")).unwrap(),
                replacement.into(),
            )
        }));
        rules
    });
    if word.is_empty() || UNCOUNTABLE.is_match(word) {
        return word.into();
    }
    for (pattern, replacement) in RULES.iter() {
        if pattern.is_match(word) {
            return pattern.replace(word, replacement.as_str()).into_owned();
        }
    }
    word.into()
}
pub fn tableize(name: &str) -> String {
    pluralize(&underscore(name))
}

#[cfg(test)]
mod schema_tests {
    #[test]
    fn tableize_and_pluralize_match_original_active_support() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/schema-upstream/inflections.json"
        )))
        .unwrap();
        for row in fixture["cases"].as_array().unwrap() {
            let source = row["source"].as_str().unwrap();
            assert_eq!(
                super::pluralize(source),
                row["pluralize"].as_str().unwrap(),
                "pluralize {source:?}"
            );
            assert_eq!(
                super::tableize(source),
                row["tableize"].as_str().unwrap(),
                "tableize {source:?}"
            );
        }
    }
}
