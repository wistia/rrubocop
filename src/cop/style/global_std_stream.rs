//! Style/GlobalStdStream — prefer $stdout over STDOUT.

use tree_sitter::Node;

use crate::cop::shared::node_bytes;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct GlobalStdStream;

impl Cop for GlobalStdStream {
    fn name(&self) -> &'static str {
        "Style/GlobalStdStream"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["constant"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let name = node_bytes(source, node);
        if name != b"STDOUT" && name != b"STDERR" && name != b"STDIN" {
            return;
        }
        let gvar = format!("${}", String::from_utf8_lossy(name).to_ascii_lowercase());
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut diag = self.diagnostic(
            source,
            line,
            col,
            format!(
                "Use `{gvar}` instead of `{}`.",
                String::from_utf8_lossy(name)
            ),
        );
        if let Some(corr) = corrections.as_mut() {
            corr.push(Correction {
                start: node.start_byte(),
                end: node.end_byte(),
                replacement: gvar,
                cop_name: self.name(),
                cop_index: 0,
                owner: None,
            });
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}
