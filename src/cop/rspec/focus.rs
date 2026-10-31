//! RSpec/Focus — breadth-first tree-sitter port.

use tree_sitter::Node;

use crate::cop::shared::{call_method_name, method_node, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct Focus;

/// `Some(replacement)` when autocorrectable; `Some("")` for report-only `focus`.
fn unfocus_repl(method: &[u8]) -> Option<&'static str> {
    match method {
        b"fdescribe" => Some("describe"),
        b"fcontext" => Some("context"),
        b"fit" => Some("it"),
        b"fexample" => Some("example"),
        b"fspecify" => Some("specify"),
        b"focus" => Some(""),
        _ => None,
    }
}

fn focus_target<'a>(source: &SourceFile, node: Node<'a>) -> Option<(Node<'a>, &'static str)> {
    if node.child_by_field_name("receiver").is_some() {
        return None;
    }
    let Some(repl) = call_method_name(source, node).and_then(unfocus_repl) else {
        return None;
    };
    Some((method_node(node).unwrap_or(node), repl))
}

impl Cop for Focus {
    fn name(&self) -> &'static str {
        "RSpec/Focus"
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*_spec.rb", "**/spec/**/*"]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "command"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some((meth, repl)) = focus_target(source, node) else {
            return;
        };
        let (line, col) = source.offset_to_line_col(meth.start_byte());
        let mut diag = self.diagnostic(source, line, col, "Focused spec found.".into());
        if !repl.is_empty()
            && push_replace(
                &mut corrections,
                meth.start_byte(),
                meth.end_byte(),
                repl,
                self.name(),
            )
        {
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Focus, "cops/rspec/focus");
}
