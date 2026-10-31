//! Style/WhileUntilDo
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct WhileUntilDo;

impl Cop for WhileUntilDo {
    fn name(&self) -> &'static str {
        "Style/WhileUntilDo"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["while", "until"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _c: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(do_node) = find_do_keyword(node) else {
            return;
        };
        let (l, c) = source.offset_to_line_col(do_node.start_byte());
        let mut diag =
            self.diagnostic(source, l, c, "Do not use `do` with `while`/`until`.".into());
        if let Some(corr) = corrections.as_mut() {
            let mut start = do_node.start_byte();
            let bytes = source.as_bytes();
            if start > 0 && matches!(bytes[start - 1], b' ' | b'\t') {
                start -= 1;
            }
            corr.push(Correction {
                start,
                end: do_node.end_byte(),
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

fn find_do_keyword(node: Node<'_>) -> Option<Node<'_>> {
    let mut cur = node.walk();
    for ch in node.children(&mut cur) {
        if is_do_token(ch) {
            return Some(ch);
        }
        // Named `do` wraps the body in some grammar versions — find the keyword token.
        if let Some(tok) = do_keyword_in(ch) {
            return Some(tok);
        }
    }
    None
}

fn is_do_token(ch: Node<'_>) -> bool {
    !ch.is_named() && ch.kind() == "do"
}

fn do_keyword_in(ch: Node<'_>) -> Option<Node<'_>> {
    if ch.kind() != "do" {
        return None;
    }
    let mut c2 = ch.walk();
    ch.children(&mut c2).find(|t| is_do_token(*t))
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(WhileUntilDo, "cops/style/while_until_do");
}
