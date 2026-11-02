use super::*;
use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::EnabledState;
use std::path::Path;

#[test]
fn interacting_custom_corrections_keep_reviewed_output() {
    let cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/deep-cop-composition.json"
    )))
    .unwrap();
    let registry = CopRegistry::default_registry();
    for case in cases.as_array().unwrap() {
        let config = configuration(case);
        let only = case["cops"]
            .as_array()
            .unwrap()
            .iter()
            .map(|name| name.as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        for (name, mode) in [
            ("off", AutocorrectMode::Off),
            ("safe", AutocorrectMode::Safe),
            ("all", AutocorrectMode::All),
        ] {
            let mut bytes = case["source"].as_str().unwrap().as_bytes().to_vec();
            crate::linter::lint_bytes_autocorrect(
                Path::new("probe.rb"),
                &mut bytes,
                &config,
                &registry,
                &CopFilterSet::build(&config, &registry),
                Some(&only),
                &[],
                mode,
                false,
            )
            .unwrap();
            assert_eq!(
                bytes,
                case["expected"][name].as_str().unwrap().as_bytes(),
                "{} {name}",
                case["name"]
            );
        }
    }
}

fn configuration(case: &serde_json::Value) -> ResolvedConfig {
    let mut config = ResolvedConfig::empty();
    config.target_ruby_version = Some(3.4);
    for name in case["cops"].as_array().unwrap() {
        let mut cop = CopConfig {
            enabled: EnabledState::True,
            ..CopConfig::default()
        };
        cop.options.insert("AutoCorrect".into(), true.into());
        config
            .cop_configs
            .insert(name.as_str().unwrap().into(), cop);
    }
    config
}
