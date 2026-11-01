//! Offense finalization (message annotator / source highlight).

use crate::config::ResolvedConfig;
use crate::cop::CopConfig;
use crate::cop::registry::CopRegistry;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub(super) fn finalize_offenses(
    source: &SourceFile,
    config: &ResolvedConfig,
    registry: &CopRegistry,
    diagnostics: &mut [Diagnostic],
) {
    for d in diagnostics.iter_mut() {
        enrich_offense(source, config, registry, d);
    }
}

fn enrich_offense(
    source: &SourceFile,
    config: &ResolvedConfig,
    registry: &CopRegistry,
    d: &mut Diagnostic,
) {
    if let Some(cop) = registry.get(d.cop_name.as_str()) {
        if !cop.supports_autocorrect() {
            d.correctable = false;
        }
    }
    fill_source_highlight(source, d);
    let cop_cfg = config.cop_config(&d.cop_name);
    d.message = crate::diagnostic::annotate_offense_message(
        strip_cop_prefix(&d.message, &d.cop_name),
        &d.cop_name,
        config.display_cop_names,
        config.extra_details,
        cop_cfg.options.get("Details").and_then(|v| v.as_str()),
        config.display_style_guide,
        style_guide_url(config, &cop_cfg).as_deref(),
    );
}

fn fill_source_highlight(source: &SourceFile, d: &mut Diagnostic) {
    if d.source_line.is_empty() {
        let line = source.cached(d.location.line, || {
            SharedLine(source.line_text(d.location.line).unwrap_or("").into())
        });
        d.source_line = line.0.clone();
    }
    if d.highlight_length == 0 {
        d.highlight_length = 1;
    }
}

fn strip_cop_prefix<'a>(message: &'a str, cop_name: &str) -> &'a str {
    let prefix = format!("{cop_name}: ");
    message.strip_prefix(&prefix).unwrap_or(message)
}

fn style_guide_url(config: &ResolvedConfig, cop_cfg: &CopConfig) -> Option<String> {
    let path = style_guide_path(cop_cfg)?;
    Some(resolve_style_guide(
        config
            .style_guide_base_url
            .as_deref()
            .filter(|s| !s.is_empty()),
        path,
    ))
}

fn style_guide_path(cop_cfg: &CopConfig) -> Option<&str> {
    cop_cfg
        .options
        .get("StyleGuide")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
}

fn resolve_style_guide(base: Option<&str>, path: &str) -> String {
    base.filter(|_| !is_http_url(path))
        .map(|b| join_url(b, path))
        .unwrap_or_else(|| path.to_string())
}

fn is_http_url(path: &str) -> bool {
    path.starts_with("http://") || path.starts_with("https://")
}

fn join_url(base: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

struct SharedLine(std::sync::Arc<str>);

#[cfg(test)]
mod tests;
