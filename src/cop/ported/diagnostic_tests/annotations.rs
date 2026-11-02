#[test]
fn annotated_range_fixtures_match_the_ruby_controls() {
    let cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/diagnostic-range-controls.json"
    )))
    .unwrap();
    let fixtures: [(usize, &[u8]); 5] = [
        (
            0,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/anycable/instance_vars/range_offense.rb"
            )),
        ),
        (
            1,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/cops/rescue_must_report_error/range_offense.rb"
            )),
        ),
        (
            6,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/cops/no_setex/range_offense.rb"
            )),
        ),
        (
            10,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/lint/numbered_parameter_assignment/range_offense.rb"
            )),
        ),
        (
            13,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/security/compound_hash/range_offense.rb"
            )),
        ),
    ];
    for (index, bytes) in fixtures {
        let parsed = crate::testutil::parse_fixture(bytes);
        assert_eq!(
            parsed.source,
            cases[index]["source"].as_str().unwrap().as_bytes()
        );
        assert_eq!(
            parsed.expected.len(),
            cases[index]["expected"].as_array().unwrap().len()
        );
    }
}
