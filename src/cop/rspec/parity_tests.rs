use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::{CopConfig, CopRegistry, EnabledState};
use crate::diagnostic::Diagnostic;
use serde_json::Value;
use std::path::Path;

fn configuration(case: &Value) -> ResolvedConfig {
    let mut config = ResolvedConfig::empty();
    config.target_ruby_version = Some(3.4);
    config.display_cop_names = false;
    config.cop_configs.insert(
        case["cop"].as_str().unwrap().into(),
        CopConfig {
            enabled: EnabledState::True,
            include: vec![],
            options: serde_json::from_value(case["config"].clone()).unwrap(),
            ..CopConfig::default()
        },
    );
    config
}

fn inspect(case: &Value, source: &str, mode: AutocorrectMode) -> (String, Vec<Diagnostic>) {
    let registry = CopRegistry::default_registry();
    let config = configuration(case);
    let mut bytes = source.as_bytes().to_vec();
    let diagnostics = crate::linter::lint_bytes_autocorrect(
        Path::new(case["path"].as_str().unwrap()),
        &mut bytes,
        &config,
        &registry,
        &CopFilterSet::build(&config, &registry),
        Some(&[case["cop"].as_str().unwrap().to_string()]),
        &[],
        mode,
        false,
    )
    .unwrap();
    (String::from_utf8(bytes).unwrap(), diagnostics)
}

fn normalized(diagnostics: &[Diagnostic]) -> Value {
    let mut diagnostics: Vec<_> = diagnostics.iter().collect();
    diagnostics.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    Value::Array(diagnostics.into_iter().map(|diagnostic| serde_json::json!({
        "line": diagnostic.location.line, "column": diagnostic.location.column,
        "message": diagnostic.message, "correctable": diagnostic.correctable,
        "length": diagnostic.highlight_length, "severity": match diagnostic.severity { crate::diagnostic::Severity::Convention => "convention", crate::diagnostic::Severity::Warning => "warning", crate::diagnostic::Severity::Error => "error", crate::diagnostic::Severity::Fatal => "fatal" }
    })).collect())
}

#[test]
fn pinned_rspec_original_ruby_detection_and_correction_parity() {
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/rspec-parity/cases.json"
    ))
    .unwrap();
    let mut failures = Vec::new();
    for case in cases {
        let id = case["id"].as_str().unwrap();
        let original = case["source"].as_str().unwrap();
        let reference = &case["reference"];
        let (_, diagnostics) = inspect(&case, original, AutocorrectMode::Off);
        if normalized(&diagnostics) != reference["diagnostics"] {
            failures.push(format!(
                "{id} detect: expected {} actual {}",
                reference["diagnostics"],
                normalized(&diagnostics)
            ));
        }
        for (name, mode) in [
            ("safe", AutocorrectMode::Safe),
            ("all", AutocorrectMode::All),
        ] {
            let (after, _) = inspect(&case, original, mode);
            if after != reference[name]["source"].as_str().unwrap() {
                failures.push(format!(
                    "{id} {name}: expected {:?} actual {after:?}",
                    reference[name]["source"]
                ));
            }
            let (_, final_diagnostics) = inspect(&case, &after, AutocorrectMode::Off);
            if normalized(&final_diagnostics) != reference[name]["diagnostics"] {
                failures.push(format!(
                    "{id} {name} final diagnostics: expected {} actual {}",
                    reference[name]["diagnostics"],
                    normalized(&final_diagnostics)
                ));
            }
            let (second, _) = inspect(&case, &after, mode);
            if after != second {
                failures.push(format!("{id} {name} is not idempotent"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} parity differences:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn documented_safety_differences_preserve_source_or_correct_behavior() {
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/rspec-parity/safety-guards.json"
    ))
    .unwrap();
    for case in cases {
        let original = case["source"].as_str().unwrap();
        let (_, detected) = inspect(&case, original, AutocorrectMode::Off);
        assert_eq!(
            normalized(&detected),
            case.get("rust_diagnostics")
                .unwrap_or(&case["reference"]["diagnostics"])
                .clone(),
            "{} must report the expected guarded offense",
            case["id"]
        );
        for (name, mode) in [
            ("rust_safe", AutocorrectMode::Safe),
            ("rust_all", AutocorrectMode::All),
        ] {
            let (after, _) = inspect(&case, original, mode);
            let expected = case
                .get(name)
                .and_then(|value| value.as_str())
                .unwrap_or(original);
            assert_eq!(
                after, expected,
                "{} must preserve source or original runtime behavior",
                case["id"]
            );
            let (_, final_diagnostics) = inspect(&case, &after, AutocorrectMode::Off);
            let expected_final = if after == original {
                case.get("rust_diagnostics")
                    .unwrap_or(&case["reference"]["diagnostics"])
                    .clone()
            } else {
                serde_json::json!([])
            };
            assert_eq!(
                normalized(&final_diagnostics),
                expected_final,
                "{} guarded final offense state",
                case["id"]
            );
            let (second, _) = inspect(&case, &after, mode);
            assert_eq!(
                after, second,
                "{} guarded correction is idempotent",
                case["id"]
            );
        }
    }
}

#[test]
fn new_rspec_rules_do_not_lint_unrelated_application_files() {
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/rspec-parity/cases.json"
    ))
    .unwrap();
    for mut case in cases {
        case["path"] = "/tmp/app/services/example.rb".into();
        let source = case["source"].as_str().unwrap();
        let (after, diagnostics) = inspect(&case, source, AutocorrectMode::All);
        assert!(
            diagnostics.is_empty(),
            "{} must honor spec-file scope",
            case["id"]
        );
        assert_eq!(after, source);
    }
}

#[test]
fn explicit_autocorrection_disable_preserves_sources() {
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/rspec-parity/cases.json"
    ))
    .unwrap();
    let mut checked = std::collections::HashSet::new();
    for mut case in cases {
        let original = case["source"].as_str().unwrap().to_string();
        if case["reference"]["all"]["source"].as_str() == Some(&original)
            || !checked.insert(case["cop"].as_str().unwrap().to_string())
        {
            continue;
        }
        case["config"]["AutoCorrect"] = "disabled".into();
        for mode in [AutocorrectMode::Safe, AutocorrectMode::All] {
            let (after, diagnostics) = inspect(&case, &original, mode);
            assert_eq!(
                after, original,
                "{} must honor AutoCorrect: disabled",
                case["id"]
            );
            assert!(!diagnostics.is_empty());
        }
    }
    assert_eq!(checked.len(), 50);
}

#[test]
fn original_rule_errors_have_safe_native_results() {
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/rspec-parity/original-rule-errors.json"
    ))
    .unwrap();
    for case in cases {
        let original = case["source"].as_str().unwrap();
        for (name, mode) in [
            ("source", AutocorrectMode::Off),
            ("rust_safe", AutocorrectMode::Safe),
            ("rust_all", AutocorrectMode::All),
        ] {
            let (after, diagnostics) = inspect(&case, original, mode);
            let expected = case.get(name).and_then(Value::as_str).unwrap_or(original);
            assert_eq!(after, expected, "{}", case["id"]);
            let expected_diagnostics = if after == original {
                case.get("native_diagnostics")
            } else {
                case.get("native_final_diagnostics")
            };
            if matches!(mode, AutocorrectMode::Off) || after == original {
                assert_eq!(
                    normalized(&diagnostics),
                    expected_diagnostics
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!([])),
                    "{}",
                    case["id"]
                );
            } else {
                let (_, final_diagnostics) = inspect(&case, &after, AutocorrectMode::Off);
                assert_eq!(
                    normalized(&final_diagnostics),
                    expected_diagnostics
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!([])),
                    "{}",
                    case["id"]
                );
                assert_eq!(inspect(&case, &after, mode).0, after);
            }
        }
    }
}
