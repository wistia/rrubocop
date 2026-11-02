use super::*;
use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::EnabledState;
use std::io::Write;

#[test]
#[ignore = "batch probe for scripts/custom_parity_audit.py"]
fn custom_parity_audit_probe() {
    let input = std::env::var("RRUBOCOP_AUDIT_INPUT").unwrap();
    let output = std::env::var("RRUBOCOP_AUDIT_OUTPUT").unwrap();
    let cases: Vec<serde_json::Value> =
        serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
    let mut output = std::io::BufWriter::new(std::fs::File::create(output).unwrap());
    let registry = CopRegistry::default_registry();
    for value in cases {
        write_case(&registry, &value, &mut output);
    }
}

fn write_case(registry: &CopRegistry, value: &serde_json::Value, output: &mut impl Write) {
    let case: Case = serde_json::from_value(value.clone()).unwrap();
    let config = configuration(&case);
    for (name, mode) in [
        ("off", AutocorrectMode::Off),
        ("safe", AutocorrectMode::Safe),
        ("all", AutocorrectMode::All),
    ] {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            inspect(registry, &case, &config, mode)
        }))
        .unwrap_or_else(
            |_| serde_json::json!({"error": "native cop panicked", "source": case.source}),
        );
        serde_json::to_writer(
            &mut *output,
            &serde_json::json!({
                "id": value["id"], "mode": name, "result": result,
            }),
        )
        .unwrap();
        writeln!(output).unwrap();
    }
}

fn configuration(case: &Case) -> ResolvedConfig {
    let mut config = ResolvedConfig::empty();
    config.target_ruby_version = Some(3.4);
    config.display_cop_names = false;
    config.cop_configs.insert(
        case.cop.clone(),
        CopConfig {
            enabled: EnabledState::True,
            severity: case
                .config
                .get("Severity")
                .and_then(|value| value.as_str())
                .and_then(crate::diagnostic::Severity::from_str),
            include: vec!["**/*".into()],
            options: case.config.clone(),
            ..CopConfig::default()
        },
    );
    config
}

fn inspect(
    registry: &CopRegistry,
    case: &Case,
    config: &ResolvedConfig,
    mode: AutocorrectMode,
) -> serde_json::Value {
    let mut bytes = case.source.as_bytes().to_vec();
    let path = case.path.replace(
        "$PROJECT",
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ported_project"),
    );
    let result = crate::linter::lint_bytes_autocorrect(
        std::path::Path::new(&path),
        &mut bytes,
        config,
        registry,
        &CopFilterSet::build(config, registry),
        Some(std::slice::from_ref(&case.cop)),
        &[],
        mode,
        false,
    );
    match result {
        Ok(diagnostics) => {
            serde_json::json!({"source": String::from_utf8(bytes).unwrap(), "diagnostics": diagnostics})
        }
        Err(error) => {
            serde_json::json!({"error": error.to_string(), "source": String::from_utf8_lossy(&bytes)})
        }
    }
}
