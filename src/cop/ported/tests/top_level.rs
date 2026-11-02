use super::*;

#[test]
fn parenthesized_top_level_checks_match_pinned_ruby() {
    let registry = CopRegistry::default_registry();
    let cases: Vec<Case> = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/top-level-parentheses-parity.json"
    ))
    .unwrap();
    for (index, case) in cases.iter().enumerate() {
        assert!(
            case_difference(&registry, index, case).is_none(),
            "{}",
            case.source
        );
    }
}

#[test]
fn top_level_parentheses_annotations_match_reference_inputs() {
    for bytes in [
        include_bytes!("../../../../tests/fixtures/cops/style/top_level_method_definition/parentheses_parity.rb").as_slice(),
        include_bytes!("../../../../tests/fixtures/cops/style/mixin_usage/parentheses_parity.rb").as_slice(),
    ] {
        let parsed = crate::testutil::parse_fixture(bytes);
        assert_eq!(parsed.expected.len(), 1);
        assert!(parsed.source.starts_with(b"("));
    }
}

#[test]
fn empty_when_semicolons_match_pinned_ruby() {
    let registry = CopRegistry::default_registry();
    let cases: Vec<Case> = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/empty-when-semicolons.json"
    ))
    .unwrap();
    for case in cases {
        runner::assert_case(&registry, &case);
    }
}
