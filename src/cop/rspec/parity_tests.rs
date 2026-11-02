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
            include: vec!["**/*".into()],
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
    Value::Array(diagnostics.iter().map(|diagnostic| serde_json::json!({
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
