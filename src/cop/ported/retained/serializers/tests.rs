use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::{CopConfig, CopRegistry, EnabledState};
use std::path::Path;

#[test]
fn serializers_never_move_methods_even_when_correction_is_enabled() {
    for cop in [
        "Serializers/PreferAttributeBlockOverAttributeAndMethod",
        "Serializers/PreferAttributeBlockOverAttributesAndMethods",
    ] {
        for mode in [AutocorrectMode::Safe, AutocorrectMode::All] {
            for source in hazardous_methods(cop) {
                assert_preserved(cop, mode, &source);
            }
        }
    }
}
fn hazardous_methods(cop: &str) -> Vec<String> {
    let declaration = if cop.ends_with("AttributesAndMethods") {
        "attributes"
    } else {
        "attribute"
    };
    [
        "def name\n    return \"name\"\n  end",
        "def name(prefix = \"prefix\")\n    prefix + object.name\n  end",
        "def  name\n    object.name\n  end",
        "def name\n    super\n  end",
        "def name\n    object.name\n  end\n  def other\n    name\n  end",
    ]
    .iter()
    .map(|method| format!("class S\n  {declaration} :name\n  {method}\nend\n"))
    .collect()
}
#[test]
fn conditional_serializer_declarations_are_not_removed() {
    for mode in [AutocorrectMode::Safe, AutocorrectMode::All] {
        assert_preserved(
            "Serializers/PreferAttributeBlockOverAttributesAndMethods",
            mode,
            "class S\n  attributes :name if enabled?\n  def name\n    object.name\n  end\nend\n",
        );
    }
}
fn assert_preserved(cop: &str, mode: AutocorrectMode, source: &str) {
    let config = enabled_config(cop);
    let registry = CopRegistry::default_registry();
    assert!(!registry.get(cop).unwrap().supports_autocorrect());
    let mut bytes = source.as_bytes().to_vec();
    let diagnostics = crate::linter::lint_bytes_autocorrect(
        Path::new("serializer.rb"),
        &mut bytes,
        &config,
        &registry,
        &CopFilterSet::build(&config, &registry),
        Some(&[cop.into()]),
        &[],
        mode,
        false,
    )
    .unwrap();
    assert_eq!(bytes, source.as_bytes());
    assert_eq!(diagnostics.len(), 1, "{cop}: {diagnostics:?}");
    assert!(!diagnostics[0].correctable);
    assert!(!diagnostics[0].corrected);
}
fn enabled_config(cop: &str) -> ResolvedConfig {
    let mut config = ResolvedConfig::empty();
    config.cop_configs.insert(
        cop.into(),
        CopConfig {
            enabled: EnabledState::True,
            options: std::collections::HashMap::from([("AutoCorrect".into(), true.into())]),
            ..CopConfig::default()
        },
    );
    config
}
