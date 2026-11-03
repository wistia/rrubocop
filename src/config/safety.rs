//! Safety validation before gem defaults and effective-cop options are applied.
use super::types::SafetySettings;
use anyhow::{Result, bail};
use serde_yml::Value;
use std::path::Path;

pub(crate) fn capture(section: &str, value: &Value) -> Option<SafetySettings> {
    let map = value.as_mapping()?;
    let safe = map.get(Value::String("Safe".into())).cloned();
    let safe_autocorrect = map.get(Value::String("SafeAutoCorrect".into())).cloned();
    (safe.is_some() || safe_autocorrect.is_some()).then(|| SafetySettings {
        section: section.into(),
        safe,
        safe_autocorrect,
    })
}

pub(crate) fn merge(base: &mut Vec<SafetySettings>, overlay: &[SafetySettings]) {
    for options in overlay {
        if let Some(current) = base
            .iter_mut()
            .find(|current| current.section == options.section)
        {
            if options.safe.is_some() {
                current.safe = options.safe.clone();
            }
            if options.safe_autocorrect.is_some() {
                current.safe_autocorrect = options.safe_autocorrect.clone();
            }
        } else {
            base.push(options.clone());
        }
    }
}

pub(crate) fn validate(settings: &[SafetySettings], path: &Path) -> Result<()> {
    for options in settings {
        if options.safe.as_ref().and_then(Value::as_bool) == Some(false)
            && options.safe_autocorrect.as_ref().and_then(Value::as_bool) == Some(true)
        {
            bail!(
                "Unsafe cops cannot have a safe autocorrection (section {} in {})",
                options.section,
                path.display()
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::cli::AutocorrectMode;
    use crate::config::{CopFilterSet, load_config};
    use crate::cop::CopRegistry;
    use serde_json::Value;
    use std::fs;

    #[test]
    fn original_file_and_inheritance_safety_matrix() {
        let cases: Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/config-safety-validation.json"
        )))
        .unwrap();
        let registry = CopRegistry::default_registry();
        for case in cases.as_array().unwrap() {
            let dir = tempfile::tempdir().unwrap();
            fs::write(
                dir.path().join("Gemfile.lock"),
                "GEM\n  specs:\n    rubocop (1.65.1)\n    rubocop-performance (1.21.1)\n",
            )
            .unwrap();
            for (index, parent) in case["parent_sources"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
            {
                fs::write(
                    dir.path().join(format!("parent-{index}.yml")),
                    parent.as_str().unwrap(),
                )
                .unwrap();
            }
            let path = dir.path().join("config.yml");
            fs::write(&path, case["config_source"].as_str().unwrap()).unwrap();
            let loaded = load_config(Some(&path), Some(dir.path()), None);
            if case["expected"]["-a"]["exit"] == 2 {
                let error = loaded.expect_err(case["id"].as_str().unwrap());
                assert!(
                    format!("{error:#}").contains("Unsafe cops cannot have a safe autocorrection"),
                    "{} {error:#}",
                    case["id"]
                );
                let section = case["expected"]["-a"]["error_section"].as_str().unwrap();
                let file = case["expected"]["-a"]["error_file"].as_str().unwrap();
                assert!(
                    format!("{error:#}").contains(&format!("section {section} in ")),
                    "{} {error:#}",
                    case["id"]
                );
                assert!(
                    format!("{error:#}").contains(file),
                    "{} {error:#}",
                    case["id"]
                );
                continue;
            }
            let config = loaded.unwrap();
            for (label, mode) in [("-a", AutocorrectMode::Safe), ("-A", AutocorrectMode::All)] {
                let mut source = case["source"].as_str().unwrap().as_bytes().to_vec();
                let diagnostics = crate::linter::lint_bytes_autocorrect(
                    &dir.path().join("input.rb"),
                    &mut source,
                    &config,
                    &registry,
                    &CopFilterSet::build(&config, &registry),
                    Some(&["Style/TrailingCommaInBlockArgs".into()]),
                    &[],
                    mode,
                    false,
                )
                .unwrap();
                assert_eq!(
                    source,
                    case["expected"][label]["bytes"]
                        .as_str()
                        .unwrap()
                        .as_bytes(),
                    "{} {label}",
                    case["id"]
                );
                assert_eq!(diagnostics.len(), 1, "{} {label}", case["id"]);
            }
        }
    }
}
