use super::super::context::*;
use tree_sitter::Node;
pub(super) fn deprecated(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if nested_scope(n) {
        return vec![];
    }
    let Some(entry) = deprecated_constant(c, n) else {
        return vec![];
    };
    let version = entry.get("DeprecatedVersion").and_then(|v| v.as_str());
    if before_deprecation(c, version) {
        return vec![];
    }
    vec![finding(c, n, entry, version)]
}
fn nested_scope(n: Node<'_>) -> bool {
    n.parent()
        .is_some_and(|p| p.kind() == "scope_resolution" && field(p, "name") == Some(n))
}
fn deprecated_constant<'a>(c: &'a Context<'_>, n: Node<'_>) -> Option<&'a serde_yml::Value> {
    c.config
        .options
        .get("DeprecatedConstants")?
        .as_mapping()?
        .get(serde_yml::Value::String(
            c.text(n).trim_start_matches("::").into(),
        ))
}
fn before_deprecation(c: &Context<'_>, version: Option<&str>) -> bool {
    version
        .and_then(|v| v.parse::<f64>().ok())
        .is_some_and(|v| c.config.get_f64("TargetRubyVersion", 2.7) < v)
}
fn finding(
    c: &Context<'_>,
    n: Node<'_>,
    entry: &serde_yml::Value,
    version: Option<&str>,
) -> Finding {
    let suffix = version
        .map(|v| format!(", deprecated since Ruby {v}"))
        .unwrap_or_default();
    let raw = c.text(n);
    match entry.get("Alternative").and_then(|v| v.as_str()) {
        Some(alt) => Finding::new(n, format!("Use `{alt}` instead of `{raw}`{suffix}."))
            .replace_with(c, n, || alt),
        None => Finding::new(n, format!("Do not use `{raw}`{suffix}.")),
    }
}
