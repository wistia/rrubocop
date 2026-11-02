//! Option-injection helpers for ResolvedConfig::cop_config.

use std::collections::HashMap;

use serde_yml::Value;

use crate::cop::{CopConfig, EnabledState};

use super::ResolvedConfig;

pub(crate) fn inject_f64(opts: &mut HashMap<String, Value>, key: &str, val: Option<f64>) {
    if let Some(v) = val {
        opts.entry(key.to_string())
            .or_insert_with(|| Value::Number(serde_yml::Number::from(v)));
    }
}

pub(crate) fn inject_bool(opts: &mut HashMap<String, Value>, key: &str, val: bool) {
    opts.entry(key.to_string())
        .or_insert_with(|| Value::Bool(val));
}

pub(crate) fn inject_sibling_str(
    opts: &mut HashMap<String, Value>,
    configs: &HashMap<String, CopConfig>,
    sibling: &str,
    sibling_key: &str,
    dest_key: &str,
    default: &str,
) {
    let style = configs
        .get(sibling)
        .and_then(|cc| cc.options.get(sibling_key))
        .and_then(|v| v.as_str())
        .unwrap_or(default);
    opts.entry(dest_key.to_string())
        .or_insert_with(|| Value::String(style.to_string()));
}

pub(crate) fn sibling_enabled(configs: &HashMap<String, CopConfig>, sibling: &str) -> bool {
    configs
        .get(sibling)
        .map(|cc| !matches!(cc.enabled, EnabledState::False))
        .unwrap_or(true)
}

pub(crate) use super::resolved_versions::inject_globals;

pub(crate) fn inject_rack_version(cfg: &ResolvedConfig, name: &str, config: &mut CopConfig) {
    if !matches!(
        name,
        "Rails/HttpStatusNameConsistency" | "RSpecRails/HttpStatusNameConsistency"
    ) {
        return;
    }
    inject_f64(&mut config.options, "__RackVersion", cfg.rack_version);
}

const LINE_LENGTH_COPS: &[&str] = &[
    "Style/IfUnlessModifier",
    "Style/WhileUntilModifier",
    "Style/ConditionalAssignment",
    "Style/GuardClause",
    "Style/SoleNestedConditional",
    "Style/MultilineMethodSignature",
    "Layout/RedundantLineBreak",
];

pub(crate) fn inject_line_length(cfg: &ResolvedConfig, name: &str, config: &mut CopConfig) {
    if !LINE_LENGTH_COPS.contains(&name) {
        return;
    }
    let ll = cfg.cop_configs.get("Layout/LineLength");
    inject_line_length_max(ll, config);
    inject_line_length_enabled(ll, config);
}

fn inject_line_length_max(ll: Option<&CopConfig>, config: &mut CopConfig) {
    if config.options.contains_key("MaxLineLength") {
        return;
    }
    config.options.insert(
        "MaxLineLength".to_string(),
        Value::Number(serde_yml::Number::from(
            ll.and_then(|cc| cc.options.get("Max"))
                .and_then(|v| v.as_u64())
                .unwrap_or(120),
        )),
    );
}

fn inject_line_length_enabled(ll: Option<&CopConfig>, config: &mut CopConfig) {
    if config.options.contains_key("LineLengthEnabled") {
        return;
    }
    config.options.insert(
        "LineLengthEnabled".to_string(),
        Value::Bool(
            ll.map(|cc| !matches!(cc.enabled, EnabledState::False))
                .unwrap_or(true),
        ),
    );
}

pub(crate) fn inject_redundant_line_break(
    cfg: &ResolvedConfig,
    name: &str,
    config: &mut CopConfig,
) {
    if name != "Layout/RedundantLineBreak" {
        return;
    }
    if config.options.contains_key("SingleLineBlockChainEnabled") {
        return;
    }
    config.options.insert(
        "SingleLineBlockChainEnabled".to_string(),
        Value::Bool(sibling_enabled(
            &cfg.cop_configs,
            "Layout/SingleLineBlockChain",
        )),
    );
}

const AS_EXTENSION_COPS: &[&str] = &[
    "Lint/DuplicateMethods",
    "Lint/UselessAccessModifier",
    "Style/ArrayIntersect",
    "Style/HashExcept",
    "Style/CollectionQuerying",
    "Style/RedundantFilterChain",
];

pub(crate) fn inject_active_support(cfg: &ResolvedConfig, name: &str, config: &mut CopConfig) {
    if !AS_EXTENSION_COPS.contains(&name) {
        return;
    }
    inject_bool(
        &mut config.options,
        "ActiveSupportExtensionsEnabled",
        cfg.active_support_extensions_enabled,
    );
}

pub(crate) fn inject_hash_alignment(cfg: &ResolvedConfig, name: &str, config: &mut CopConfig) {
    if name != "Layout/HashAlignment" {
        return;
    }
    inject_bool(
        &mut config.options,
        "ArgumentAlignmentEnabled",
        sibling_enabled(&cfg.cop_configs, "Layout/ArgumentAlignment"),
    );
    inject_sibling_str(
        &mut config.options,
        &cfg.cop_configs,
        "Layout/ArgumentAlignment",
        "EnforcedStyle",
        "ArgumentAlignmentStyle",
        "with_first_argument",
    );
}

pub(crate) fn inject_first_argument(cfg: &ResolvedConfig, name: &str, config: &mut CopConfig) {
    if name != "Layout/FirstArgumentIndentation" {
        return;
    }
    inject_sibling_str(
        &mut config.options,
        &cfg.cop_configs,
        "Layout/ArgumentAlignment",
        "EnforcedStyle",
        "ArgumentAlignmentStyle",
        "with_first_argument",
    );
    inject_bool(
        &mut config.options,
        "FirstMethodArgumentLineBreakEnabled",
        sibling_enabled(&cfg.cop_configs, "Layout/FirstMethodArgumentLineBreak"),
    );
}

fn argument_alignment_fixed(configs: &HashMap<String, CopConfig>) -> bool {
    sibling_enabled(configs, "Layout/ArgumentAlignment")
        && configs
            .get("Layout/ArgumentAlignment")
            .and_then(|cc| cc.options.get("EnforcedStyle"))
            .and_then(|v| v.as_str())
            == Some("with_fixed_indentation")
}

pub(crate) fn inject_first_hash_indent(cfg: &ResolvedConfig, name: &str, config: &mut CopConfig) {
    if name != "Layout/FirstHashElementIndentation" {
        return;
    }
    for (key, default) in [
        ("EnforcedColonStyle", "key"),
        ("EnforcedHashRocketStyle", "key"),
    ] {
        config.options.entry(key.to_string()).or_insert(
            cfg.cop_configs
                .get("Layout/HashAlignment")
                .and_then(|cc| cc.options.get(key))
                .cloned()
                .unwrap_or_else(|| Value::String(default.to_string())),
        );
    }
    inject_bool(
        &mut config.options,
        "FixedArgumentIndentation",
        argument_alignment_fixed(&cfg.cop_configs),
    );
}

pub(crate) fn inject_end_alignment(cfg: &ResolvedConfig, name: &str, config: &mut CopConfig) {
    if name != "Layout/ElseAlignment" && name != "Layout/IndentationWidth" {
        return;
    }
    inject_sibling_str(
        &mut config.options,
        &cfg.cop_configs,
        "Layout/EndAlignment",
        "EnforcedStyleAlignWith",
        "EndAlignmentStyle",
        "keyword",
    );
}

pub(crate) fn inject_rescue_ensure_alignment(
    cfg: &ResolvedConfig,
    name: &str,
    config: &mut CopConfig,
) {
    if name != "Layout/RescueEnsureAlignment" {
        return;
    }
    inject_sibling_str(
        &mut config.options,
        &cfg.cop_configs,
        "Layout/BeginEndAlignment",
        "EnforcedStyleAlignWith",
        "BeginEndAlignmentStyle",
        "begin",
    );
    inject_bool(
        &mut config.options,
        "BeginEndAlignmentEnabled",
        sibling_enabled(&cfg.cop_configs, "Layout/BeginEndAlignment"),
    );
}

pub(crate) fn inject_indentation_width(cfg: &ResolvedConfig, name: &str, config: &mut CopConfig) {
    if name != "Layout/IndentationWidth" {
        return;
    }
    inject_sibling_str(
        &mut config.options,
        &cfg.cop_configs,
        "Layout/IndentationConsistency",
        "EnforcedStyle",
        "IndentationConsistencyStyle",
        "normal",
    );
    inject_sibling_str(
        &mut config.options,
        &cfg.cop_configs,
        "Layout/AccessModifierIndentation",
        "EnforcedStyle",
        "AccessModifierIndentationStyle",
        "indent",
    );
    inject_sibling_str(
        &mut config.options,
        &cfg.cop_configs,
        "Layout/IndentationStyle",
        "EnforcedStyle",
        "IndentationStyleEnforced",
        "spaces",
    );
    inject_sibling_str(
        &mut config.options,
        &cfg.cop_configs,
        "Layout/DefEndAlignment",
        "EnforcedStyleAlignWith",
        "DefEndAlignmentStyle",
        "start_of_line",
    );
}

pub(crate) fn inject_space_after_comma(cfg: &ResolvedConfig, name: &str, config: &mut CopConfig) {
    if name != "Layout/SpaceAfterComma" {
        return;
    }
    inject_sibling_str(
        &mut config.options,
        &cfg.cop_configs,
        "Layout/SpaceInsideHashLiteralBraces",
        "EnforcedStyle",
        "__SpaceInsideHashBracesStyle",
        "space",
    );
}

pub(crate) fn inject_missing_else(cfg: &ResolvedConfig, name: &str, config: &mut CopConfig) {
    if name != "Style/MissingElse" {
        return;
    }
    inject_bool(
        &mut config.options,
        "UnlessElseEnabled",
        sibling_enabled(&cfg.cop_configs, "Style/UnlessElse"),
    );
    inject_sibling_str(
        &mut config.options,
        &cfg.cop_configs,
        "Style/EmptyElse",
        "EnforcedStyle",
        "EmptyElseStyle",
        "both",
    );
}

pub(crate) fn inject_quoted_symbols(cfg: &ResolvedConfig, name: &str, config: &mut CopConfig) {
    if name != "Style/QuotedSymbols" {
        return;
    }
    inject_sibling_str(
        &mut config.options,
        &cfg.cop_configs,
        "Style/StringLiterals",
        "EnforcedStyle",
        "StringLiteralsEnforcedStyle",
        "single_quotes",
    );
}

/// RuboCop CommentConfig seeds `disable_count` with config-disabled cops so
/// `# rubocop:enable Layout/LineLength` is not "extra" when LineLength is off.
pub(crate) fn inject_config_disabled_cops(
    cfg: &ResolvedConfig,
    name: &str,
    config: &mut CopConfig,
) {
    if name != "Lint/RedundantCopEnableDirective" {
        return;
    }
    config
        .options
        .entry("ConfigDisabledCops".into())
        .or_insert(Value::Sequence(
            cfg.cop_configs
                .iter()
                .filter(|(_, c)| matches!(c.enabled, EnabledState::False))
                .map(|(n, _)| Value::String(n.clone()))
                .collect(),
        ));
}

pub(crate) fn inject_operator_spacing(cfg: &ResolvedConfig, name: &str, config: &mut CopConfig) {
    if name != "Layout/SpaceAroundOperators" {
        return;
    }
    inject_sibling_str(
        &mut config.options,
        &cfg.cop_configs,
        "Layout/HashAlignment",
        "EnforcedHashRocketStyle",
        "EnforcedHashRocketStyle",
        "key",
    );
    let force_alignment = cfg
        .cop_configs
        .get("Layout/ExtraSpacing")
        .and_then(|extra| extra.options.get("ForceEqualSignAlignment"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    inject_bool(
        &mut config.options,
        "ForceEqualSignAlignment",
        force_alignment,
    );
}

pub(crate) fn inject_redundant_parentheses(
    cfg: &ResolvedConfig,
    name: &str,
    config: &mut CopConfig,
) {
    if name != "Style/RedundantParentheses" {
        return;
    }
    inject_bool(
        &mut config.options,
        "TernaryParenthesesEnabled",
        sibling_enabled(&cfg.cop_configs, "Style/TernaryParentheses"),
    );
    inject_sibling_str(
        &mut config.options,
        &cfg.cop_configs,
        "Style/TernaryParentheses",
        "EnforcedStyle",
        "TernaryParenthesesEnforcedStyle",
        "require_no_parentheses",
    );
    inject_bool(
        &mut config.options,
        "ParenthesesAroundConditionEnabled",
        sibling_enabled(&cfg.cop_configs, "Style/ParenthesesAroundCondition"),
    );
    let allow = cfg
        .cop_configs
        .get("Style/ParenthesesAroundCondition")
        .is_some_and(|condition| condition.get_bool("AllowInMultilineConditions", false));
    inject_bool(&mut config.options, "AllowInMultilineConditions", allow);
}
