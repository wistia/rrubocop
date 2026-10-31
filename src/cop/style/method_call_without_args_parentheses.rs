//! Style/MethodCallWithoutArgsParentheses.

use tree_sitter::Node;

use crate::cop::shared::call_method_name;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MethodCallWithoutArgsParentheses;

impl Cop for MethodCallWithoutArgsParentheses {
    fn name(&self) -> &'static str {
        "Style/MethodCallWithoutArgsParentheses"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(args) = empty_args(source, node) else {
            return;
        };
        report(self, source, args, diagnostics, &mut corrections);
    }
}

fn assignment_shadows(source: &SourceFile, node: Node<'_>, meth: &[u8]) -> bool {
    let mut parent = node.parent();
    while let Some(n) = parent {
        if n.kind() == "assignment"
            && n.child_by_field_name("left")
                .is_some_and(|l| crate::cop::shared::node_bytes(source, l) == meth)
        {
            return true;
        }
        if matches!(n.kind(), "method" | "singleton_method" | "class" | "module") {
            break;
        }
        parent = n.parent();
    }
    false
}

fn empty_args<'a>(source: &SourceFile, node: Node<'a>) -> Option<Node<'a>> {
    // RuboCop uses `on_send` only — `super()` / `yield()` are separate node types.
    if matches!(call_method_name(source, node), Some(b"super" | b"yield")) {
        return None;
    }
    let Some(meth) = call_method_name(source, node) else {
        return None;
    };
    // RuboCop `camel_case_method?` — `Success()`, `Failure()`, etc.
    if meth.first().is_some_and(|b| b.is_ascii_uppercase()) {
        return None;
    }
    if assignment_shadows(source, node, meth) {
        return None;
    }
    let args = node.child_by_field_name("arguments")?;
    let mut cur = args.walk();
    if args.named_children(&mut cur).next().is_some() {
        return None;
    }
    Some(args)
}

fn report(
    cop: &MethodCallWithoutArgsParentheses,
    source: &SourceFile,
    args: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(args.start_byte());
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        "Do not use parentheses for method calls with no arguments.".to_string(),
    );
    if let Some(corr) = corrections.as_mut() {
        corr.push(Correction {
            start: args.start_byte(),
            end: args.end_byte(),
            replacement: String::new(),
            cop_name: cop.name(),
            cop_index: 0,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        MethodCallWithoutArgsParentheses,
        "cops/style/method_call_without_args_parentheses"
    );
}
