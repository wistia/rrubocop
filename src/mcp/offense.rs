//! LSP-shaped offense JSON (matches RuboCop MCP / LSP diagnostics).

use serde_json::{Value, json};

use crate::diagnostic::{Diagnostic, Severity};

/// Map rrubocop severity to LSP DiagnosticSeverity.
fn lsp_severity(s: Severity) -> u8 {
    match s {
        Severity::Fatal | Severity::Error => 1,
        Severity::Warning => 2,
        Severity::Convention => 3,
    }
}

/// RuboCop docs href when the department is on docs.rubocop.org.
fn docs_href(cop_name: &str) -> Option<String> {
    let (dept, rest) = cop_name.split_once('/')?;
    let dept_l = dept.to_lowercase();
    let name_l = rest.replace('_', "").to_lowercase();
    // Core + common plugin pages live under rubocop.org cops_<dept>.
    Some(format!(
        "https://docs.rubocop.org/rubocop/cops_{dept_l}.html#{dept_l}{name_l}"
    ))
}

fn offense_message(d: &Diagnostic) -> String {
    if d.correctable {
        d.message.clone()
    } else {
        format!("{}\n\nThis offense is not autocorrectable.\n", d.message)
    }
}

/// One offense as returned by RuboCop `LSP::Runtime#offenses` JSON.
pub(crate) fn to_lsp_offense(d: &Diagnostic) -> Value {
    let line = d.location.line.saturating_sub(1);
    let start = d.location.column;
    let end = start + d.highlight_length.max(1);
    let mut obj = json!({
        "range": {
            "start": { "line": line, "character": start },
            "end": { "line": line, "character": end }
        },
        "severity": lsp_severity(d.severity),
        "source": "RuboCop",
        "code": d.cop_name,
        "message": offense_message(d),
        "data": { "correctable": d.correctable }
    });
    if let Some(href) = docs_href(&d.cop_name) {
        obj.as_object_mut()
            .unwrap()
            .insert("codeDescription".into(), json!({ "href": href }));
    }
    obj
}

pub(crate) fn offenses_json(diags: &[Diagnostic]) -> String {
    serde_json::to_string(&diags.iter().map(to_lsp_offense).collect::<Vec<_>>())
        .unwrap_or_else(|_| "[]".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_source_lines_keep_diagnostic_serde_and_lsp_contracts() {
        let diagnostic = Diagnostic {
            path: "a.rb".into(),
            location: crate::diagnostic::Location { line: 1, column: 1 },
            severity: Severity::Warning,
            cop_name: "Test/Cop".into(),
            message: "message".into(),
            corrected: false,
            correctable: true,
            source_line: "original source".into(),
            highlight_length: 2,
        };
        let restored: Diagnostic =
            serde_json::from_value(serde_json::to_value(&diagnostic).unwrap()).unwrap();
        assert_eq!(to_lsp_offense(&diagnostic), to_lsp_offense(&restored));
        let value = to_lsp_offense(&restored);
        assert_eq!(value["range"]["end"]["character"], 3);
        assert!(value.get("source_line").is_none());
    }
}
