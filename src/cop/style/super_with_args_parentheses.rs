//! Style/SuperWithArgsParentheses — super(...) needs parens.

use tree_sitter::Node;

use crate::cop::shared::{argument_nodes, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SuperWithArgsParentheses;

impl Cop for SuperWithArgsParentheses {
    fn name(&self) -> &'static str {
        "Style/SuperWithArgsParentheses"
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
        let Some(first) = super_kw(source, node) else {
            return;
        };
        if node.child_by_field_name("arguments").is_none() || node.kind() != "command" {
            return;
        }
        report(self, source, node, first, diagnostics, &mut corrections);
    }
}

fn super_kw<'a>(source: &SourceFile, node: Node<'a>) -> Option<Node<'a>> {
    let mut cur = node.walk();
    let first = node.children(&mut cur).next()?;
    if node_bytes(source, first) != b"super" {
        return None;
    }
    Some(first)
}

fn report(
    cop: &SuperWithArgsParentheses,
    source: &SourceFile,
    node: Node<'_>,
    first: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(node.start_byte());
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        "Use parentheses for `super` call with arguments.".to_string(),
    );
    if let Some(corr) = corrections.as_mut() {
        push_parens(cop, node, first, corr, &mut diag);
    }
    diagnostics.push(diag);
}

fn push_parens(
    cop: &SuperWithArgsParentheses,
    node: Node<'_>,
    first: Node<'_>,
    corr: &mut Vec<Correction>,
    diag: &mut Diagnostic,
) {
    let args = argument_nodes(node);
    let (Some(first_arg), Some(last_arg)) = (args.first(), args.last()) else {
        return;
    };
    corr.push(Correction {
        start: first.end_byte(),
        end: first_arg.start_byte(),
        replacement: "(".into(),
        cop_name: cop.name(),
        cop_index: 0,
        owner: None,
    });
    corr.push(Correction {
        start: last_arg.end_byte(),
        end: last_arg.end_byte(),
        replacement: ")".into(),
        cop_name: cop.name(),
        cop_index: 0,
        owner: None,
    });
    diag.corrected = true;
}
