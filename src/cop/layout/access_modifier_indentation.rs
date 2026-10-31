//! Layout/AccessModifierIndentation.

use tree_sitter::Node;

use crate::cop::layout::report;
use crate::cop::shared;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct AccessModifierIndentation;

fn modifier_name<'a>(source: &'a SourceFile, node: Node<'_>) -> Option<&'a [u8]> {
    if node.kind() == "identifier" {
        Some(shared::node_bytes(source, node))
    } else {
        shared::call_method_name(source, node)
    }
}

fn is_modifier(name: &[u8]) -> bool {
    matches!(
        name,
        b"private" | b"protected" | b"public" | b"module_function"
    )
}

fn enclosing_type(node: Node<'_>) -> Option<Node<'_>> {
    let mut p = node.parent();
    while let Some(n) = p {
        // `class_methods do` / similar — RuboCop treats the block as the module-like scope.
        if matches!(n.kind(), "method" | "singleton_method") {
            return None;
        }
        if matches!(n.kind(), "do_block" | "block") {
            return Some(n);
        }
        if matches!(n.kind(), "class" | "module" | "singleton_class") {
            return Some(n);
        }
        p = n.parent();
    }
    None
}

fn base_col(source: &SourceFile, enclosing: Node<'_>) -> usize {
    if matches!(enclosing.kind(), "do_block" | "block") {
        // Align with sibling method defs: body indent (first stmt), not `do` column.
        if let Some(body) = enclosing.child_by_field_name("body") {
            let mut cur = body.walk();
            if let Some(first) = body
                .named_children(&mut cur)
                .find(|c| !matches!(c.kind(), "comment" | "rescue" | "ensure"))
            {
                return shared::line_indent(source, first.start_byte());
            }
        }
        return shared::line_indent(source, enclosing.start_byte());
    }
    shared::node_col(source, enclosing)
}

fn expected_col(style: &str, base: usize, width: usize, enclosing_is_block: bool) -> usize {
    if enclosing_is_block {
        // Inside `class_methods do`, modifiers align with method defs (same indent).
        return base;
    }
    if style == "outdent" {
        base
    } else {
        base + width
    }
}

fn report_modifier(
    cop: &dyn Cop,
    source: &SourceFile,
    node: Node<'_>,
    name: &[u8],
    style: &str,
    expected: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let mod_name = String::from_utf8_lossy(name);
    let style_word = if style == "outdent" {
        "Outdent"
    } else {
        "Indent"
    };
    report::fix_indent(
        cop,
        source,
        node.start_byte(),
        format!("{style_word} access modifiers like `{mod_name}`."),
        diagnostics,
        corrections,
        shared::line_indent(source, node.start_byte()),
        expected,
    );
}

/// `obj.private` / argument identifiers are not access modifiers.
fn is_receiver_or_arg_ident(node: Node<'_>) -> bool {
    node.kind() == "identifier"
        && matches!(
            node.parent().map(|p| p.kind()),
            Some("call" | "command" | "command_call")
        )
}

fn skip_modifier(node: Node<'_>, name: &[u8]) -> bool {
    !is_modifier(name)
        || is_receiver_or_arg_ident(node)
        || shared::call_receiver(node).is_some()
        || is_method_name_part(node)
        || !is_access_modifier_form(node)
}

/// Bare `private`/`public` as a statement, or `private def` / `private :sym` calls —
/// not kwargs (`private: false`) or ordinary identifiers.
fn is_access_modifier_form(node: Node<'_>) -> bool {
    match node.kind() {
        "call" | "command" | "command_call" => {
            node.child_by_field_name("arguments").is_none_or(|args| {
                let mut cursor = args.walk();
                args.named_children(&mut cursor)
                    .all(|n| matches!(n.kind(), "method" | "singleton_method" | "comment"))
            })
        }
        "identifier" => node.parent().is_some_and(|p| {
            matches!(
                p.kind(),
                "body_statement" | "begin" | "program" | "then" | "else" | "ensure" | "rescue"
            )
        }),
        _ => false,
    }
}

fn is_method_name_part(node: Node<'_>) -> bool {
    if node.kind() != "identifier" {
        return false;
    }
    let mut p = node.parent();
    while let Some(n) = p {
        if matches!(n.kind(), "method" | "singleton_method") {
            return n.child_by_field_name("name").is_some_and(|name| {
                name.start_byte() <= node.start_byte() && name.end_byte() >= node.end_byte()
            });
        }
        p = n.parent();
    }
    false
}

impl Cop for AccessModifierIndentation {
    fn name(&self) -> &'static str {
        "Layout/AccessModifierIndentation"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "command", "identifier"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "indent");
        let width = config.get_usize("IndentationWidth", 2);
        let Some(name) = modifier_name(source, node) else {
            return;
        };
        if skip_modifier(node, name) {
            return;
        }
        let Some(enclosing) = enclosing_type(node) else {
            return;
        };
        let base = base_col(source, enclosing);
        let is_block = matches!(enclosing.kind(), "do_block" | "block");
        let expected = expected_col(style, base, width, is_block);
        if shared::node_col(source, node) == expected {
            return;
        }
        report_modifier(
            self,
            source,
            node,
            name,
            style,
            expected,
            diagnostics,
            &mut corrections,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        AccessModifierIndentation,
        "cops/layout/access_modifier_indentation"
    );
}
