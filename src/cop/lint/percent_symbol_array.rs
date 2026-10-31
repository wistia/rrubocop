use tree_sitter::Node;

use crate::cop::shared::node_text;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Lint/PercentSymbolArray — colons/commas inside %i/%I.
pub struct PercentSymbolArray;

fn strip_colons_commas(s: &str) -> String {
    let mut out = s.to_string();
    if out.ends_with(',') {
        out.pop();
    }
    if out.starts_with(':') {
        out.remove(0);
    }
    out
}

fn bad_bare_symbols<'a>(source: &SourceFile, node: Node<'a>) -> Vec<Node<'a>> {
    let mut cur = node.walk();
    node.named_children(&mut cur)
        .filter(|c| c.kind() == "bare_symbol")
        .filter(|c| {
            let t = node_text(source, *c);
            t.contains(':') || t.contains(',')
        })
        .collect()
}

fn apply_fixes(
    source: &SourceFile,
    cop_name: &'static str,
    offenders: &[Node<'_>],
    corr: &mut Vec<Correction>,
) {
    for child in offenders {
        let t = node_text(source, *child);
        let cleaned = strip_colons_commas(&t);
        if cleaned != t {
            corr.push(Correction {
                start: child.start_byte(),
                end: child.end_byte(),
                replacement: cleaned,
                cop_name,
                cop_index: 0,
                owner: None,
            });
        }
    }
}

impl Cop for PercentSymbolArray {
    fn name(&self) -> &'static str {
        "Lint/PercentSymbolArray"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["symbol_array"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let offenders = bad_bare_symbols(source, node);
        if offenders.is_empty() {
            return;
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut diag = self.diagnostic(
            source,
            line,
            col,
            "Within `%i`/`%I`, ':' and ',' are unnecessary and may be unwanted in the resulting symbols."
                .to_string(),
        );
        if let Some(corr) = corrections.as_mut() {
            apply_fixes(source, self.name(), &offenders, corr);
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}
