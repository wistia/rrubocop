//! Style/SpecialGlobalVars — prefer English names for special globals.

use tree_sitter::Node;

use crate::cop::shared::node_bytes;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SpecialGlobalVars;

const PERL: &[&[u8]] = &[
    b"$!", b"$@", b"$;", b"$,", b"$.", b"$/", b"$\\", b"$\"", b"$0", b"$*", b"$$", b"$?", b"$&",
    b"$`", b"$'", b"$+", b"$~", b"$=", b"$:",
];

const ENGLISH: &[(&[u8], &str)] = &[
    (b"$:", "$LOAD_PATH"),
    (b"$\"", "$LOADED_FEATURES"),
    (b"$!", "$ERROR_INFO"),
    (b"$@", "$ERROR_POSITION"),
    (b"$;", "$FIELD_SEPARATOR"),
    (b"$,", "$OUTPUT_FIELD_SEPARATOR"),
    (b"$/", "$INPUT_RECORD_SEPARATOR"),
    (b"$\\", "$OUTPUT_RECORD_SEPARATOR"),
    (b"$.", "$INPUT_LINE_NUMBER"),
    (b"$0", "$PROGRAM_NAME"),
    (b"$$", "$PROCESS_ID"),
    (b"$?", "$CHILD_STATUS"),
    (b"$~", "$LAST_MATCH_INFO"),
    (b"$=", "$IGNORECASE"),
    (b"$*", "$ARGV"),
    (b"$&", "$MATCH"),
    (b"$`", "$PREMATCH"),
    (b"$'", "$POSTMATCH"),
    (b"$+", "$LAST_PAREN_MATCH"),
];

fn english_name(name: &[u8]) -> Option<&'static str> {
    ENGLISH.iter().find(|(k, _)| *k == name).map(|(_, v)| *v)
}

impl Cop for SpecialGlobalVars {
    fn name(&self) -> &'static str {
        "Style/SpecialGlobalVars"
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
        let name = node_bytes(source, node);
        if !PERL.contains(&name) {
            return;
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut diag = self.diagnostic(
            source,
            line,
            col,
            "Prefer English names for special global variables.".to_string(),
        );
        if let Some(corr) = corrections.as_mut()
            && let Some(english) = english_name(name)
        {
            corr.push(Correction {
                start: node.start_byte(),
                end: node.end_byte(),
                replacement: english.to_string(),
                cop_name: self.name(),
                cop_index: 0,
                owner: None,
            });
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}
