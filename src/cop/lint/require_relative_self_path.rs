use tree_sitter::Node;

use crate::cop::shared::{argument_nodes, call_method_name, node_text};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Lint/RequireRelativeSelfPath — require_relative of own file.
pub struct RequireRelativeSelfPath;

fn string_inner(source: &SourceFile, node: Node<'_>) -> Option<String> {
    if node.kind() != "string" {
        return None;
    }
    Some(
        node_text(source, node)
            .trim_matches(|c| c == '\'' || c == '"')
            .to_string(),
    )
}

fn is_self_path(source: &SourceFile, inner: &str) -> bool {
    let stem = std::path::Path::new(source.path_str())
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    inner == stem
        || inner == format!("./{stem}")
        || inner == format!("{stem}.rb")
        || inner == format!("./{stem}.rb")
}

fn self_require_node(source: &SourceFile, node: Node<'_>) -> bool {
    if call_method_name(source, node) != Some(b"require_relative") {
        return false;
    }
    argument_nodes(node)
        .into_iter()
        .next()
        .and_then(|arg| string_inner(source, arg))
        .is_some_and(|inner| is_self_path(source, &inner))
}

fn line_span(source: &SourceFile, node: Node<'_>) -> (usize, usize) {
    let mut start = node.start_byte();
    let mut end = node.end_byte();
    let bytes = source.as_bytes();
    if end < bytes.len() && bytes[end] == b'\n' {
        end += 1;
    } else if start > 0 && bytes[start - 1] == b'\n' {
        start -= 1;
    }
    (start, end)
}

impl Cop for RequireRelativeSelfPath {
    fn name(&self) -> &'static str {
        "Lint/RequireRelativeSelfPath"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }

    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"require_relative"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !self_require_node(source, node) {
            return;
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut diag = self.diagnostic(
            source,
            line,
            col,
            "Remove the `require_relative` that requires itself.".to_string(),
        );
        if let Some(corr) = corrections.as_mut() {
            let (start, end) = line_span(source, node);
            corr.push(Correction {
                start,
                end,
                replacement: String::new(),
                cop_name: self.name(),
                cop_index: 0,
                owner: None,
            });
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}
