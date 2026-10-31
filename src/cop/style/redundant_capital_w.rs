//! Style/RedundantCapitalW — prefer `%w` when `%W` needs no interpolation.

use tree_sitter::Node;

use crate::cop::style::heuristics::matches_redundant_capital_w;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantCapitalW;

impl Cop for RedundantCapitalW {
    fn name(&self) -> &'static str {
        "Style/RedundantCapitalW"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["string_array", "array"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !matches_redundant_capital_w(source, node, config) {
            return;
        }
        if needs_capital_w(source, node) {
            return;
        }
        report(self, source, node, diagnostics, &mut corrections);
    }
}

fn needs_capital_w(source: &SourceFile, node: Node<'_>) -> bool {
    let bytes = &source.as_bytes()[node.start_byte()..node.end_byte()];
    if bytes.len() <= 4 {
        return false;
    }
    let content = &bytes[3..bytes.len().saturating_sub(1)];
    content
        .windows(2)
        .any(|w| w[0] == b'#' && (w[1] == b'{' || w[1] == b'@' || w[1] == b'$'))
        || content.contains(&b'\\')
        || content.contains(&b'\'')
}

fn report(
    cop: &RedundantCapitalW,
    source: &SourceFile,
    node: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(node.start_byte());
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        "Do not use `%W` unless interpolation is needed. If not, use `%w`.".to_string(),
    );
    if let Some(corr) = corrections.as_mut() {
        corr.push(Correction {
            start: node.start_byte() + 1,
            end: node.start_byte() + 2,
            replacement: "w".to_string(),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}
