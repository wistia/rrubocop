//! Style/PerlBackrefs — prefer Regexp.last_match over $1.

use tree_sitter::Node;

use crate::cop::shared::node_bytes;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct PerlBackrefs;

impl Cop for PerlBackrefs {
    fn name(&self) -> &'static str {
        "Style/PerlBackrefs"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["global_variable"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(digits) = digit_backref(source, node) else {
            return;
        };
        report(self, source, node, &digits, diagnostics, &mut corrections);
    }
}

fn digit_backref(source: &SourceFile, node: Node<'_>) -> Option<String> {
    let name = node_bytes(source, node);
    if name.len() < 2 || name[0] != b'$' || !name[1].is_ascii_digit() {
        return None;
    }
    let n = &name[1..];
    if !n.iter().all(|b| b.is_ascii_digit()) {
        return None;
    }
    // `$0` is the program name (Style/GlobalVars built-in), not a regexp backref.
    if n.iter().all(|&b| b == b'0') {
        return None;
    }
    Some(String::from_utf8_lossy(n).into_owned())
}

fn report(
    cop: &PerlBackrefs,
    source: &SourceFile,
    node: Node<'_>,
    digits: &str,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let preferred = format!("Regexp.last_match({digits})");
    let (line, col) = source.offset_to_line_col(node.start_byte());
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        format!("Prefer `{preferred}` over `${digits}`."),
    );
    if let Some(corr) = corrections.as_mut() {
        corr.push(Correction {
            start: node.start_byte(),
            end: node.end_byte(),
            replacement: preferred,
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(PerlBackrefs, "cops/style/perl_backrefs");
}
