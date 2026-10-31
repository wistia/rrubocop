use super::*;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub(super) fn check_pairs(cases: &[Case]) {
    let names = cases
        .iter()
        .map(|case| case.cop.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(names.len(), 98);
    for name in names {
        check_pair(cases, name);
    }
}
fn check_pair(cases: &[Case], name: &str) {
    for (positive, filename) in [(true, "offense.rb"), (false, "no_offense.rb")] {
        let parsed = parsed_fixture(name, filename);
        let original = matching_case(cases, name, positive, &parsed);
        let case = fixture_case(original, &parsed);
        assert_eq!(case.source, original.source, "{name} {filename}");
        assert_eq!(
            parsed.expected.len(),
            runner::expected(original).len(),
            "{name} {filename}"
        );
        runner::assert_case(&CopRegistry::default_registry(), &case);
    }
}
fn matching_case<'a>(
    cases: &'a [Case],
    name: &str,
    positive: bool,
    parsed: &crate::testutil::ParsedFixture,
) -> &'a Case {
    cases
        .iter()
        .find(|case| {
            case.cop == name
                && case.source.as_bytes() == parsed.source
                && !runner::expected(case).is_empty() == positive
        })
        .unwrap_or_else(|| panic!("No reference case for {name}, positive={positive}"))
}

fn fixture_case(original: &Case, parsed: &crate::testutil::ParsedFixture) -> Case {
    let mut case = original.clone();
    case.source = String::from_utf8(parsed.source.clone()).unwrap();
    case
}
fn parsed_fixture(name: &str, filename: &str) -> crate::testutil::ParsedFixture {
    crate::testutil::parse_fixture(&std::fs::read(fixture_path(name).join(filename)).unwrap())
}
fn fixture_path(name: &str) -> PathBuf {
    let (dept, cop) = name.split_once('/').unwrap();
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/cops")
        .join(dept.to_lowercase())
        .join(snake(cop))
}
fn snake(cop: &str) -> String {
    regex::Regex::new("([a-z0-9])([A-Z])")
        .unwrap()
        .replace_all(&split_acronym(cop), "${1}_${2}")
        .to_lowercase()
}
fn split_acronym(cop: &str) -> String {
    regex::Regex::new("([A-Z]+)([A-Z][a-z])")
        .unwrap()
        .replace_all(cop, "${1}_${2}")
        .into_owned()
}
