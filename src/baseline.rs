//! RuboCop gem versions this build targets for offense / config parity.
//! Keep in sync with `resources/baseline.json` (nitrocop pin set).

/// Core RuboCop version.
pub const RUBOCOP: &str = "1.91.0";

/// Extension gems (name, version), same order as nitrocop baseline docs.
pub const GEMS: &[(&str, &str)] = &[
    ("rubocop-rails", "2.37.0"),
    ("rubocop-performance", "1.27.0"),
    ("rubocop-rspec", "3.9.0"),
    ("rubocop-rspec_rails", "2.32.0"),
    ("rubocop-factory_bot", "2.28.0"),
    ("rubocop-graphql", "1.8.0"),
];

/// Shown at the top of `--help`.
pub const ABOUT: &str = concat!(
    env!("CARGO_PKG_DESCRIPTION"),
    "\nCorresponds to rubocop 1.91.0 with rubocop-rails 2.37.0, rubocop-performance 1.27.0, ",
    "rubocop-rspec 3.9.0, rubocop-rspec_rails 2.32.0, rubocop-factory_bot 2.28.0, ",
    "rubocop-graphql 1.8.0"
);

/// Shown by `-V` / `--version`.
pub const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (sha ",
    env!("RRUBOCOP_BUILD_SHA"),
    ")",
    " (corresponds to rubocop 1.91.0 with rubocop-rails 2.37.0, rubocop-performance 1.27.0, ",
    "rubocop-rspec 3.9.0, rubocop-rspec_rails 2.32.0, rubocop-factory_bot 2.28.0, ",
    "rubocop-graphql 1.8.0)"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn about_and_version_match_baseline_json() {
        assert!(VERSION.contains(concat!("sha ", env!("RRUBOCOP_BUILD_SHA"))));
        let raw: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(include_str!("resources/baseline.json")).unwrap();
        for (name, ver) in raw {
            let needle = format!("{name} {}", ver.as_str().unwrap());
            assert!(ABOUT.contains(&needle), "{needle} missing from ABOUT");
            assert!(VERSION.contains(&needle), "{needle} missing from VERSION");
        }
        assert_eq!(RUBOCOP, "1.91.0");
        assert_eq!(GEMS.len(), 6);
    }
}
