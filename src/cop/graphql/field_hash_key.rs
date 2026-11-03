//! GraphQL/FieldHashKey — prefer hash_key: over trivial object[] resolvers.

use tree_sitter::Node;

use super::helpers::{
    CALL_KINDS, DEPT_INCLUDE, enclosing_class, find_method_def, is_conflict_field_name,
    is_field_call, resolver_method_name,
};
use crate::cop::shared::{argument_nodes, call_method_name, call_receiver, named_kids, node_text};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct FieldHashKey;

impl Cop for FieldHashKey {
    fn name(&self) -> &'static str {
        "GraphQL/FieldHashKey"
    }

    fn default_include(&self) -> &'static [&'static str] {
        DEPT_INCLUDE
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        CALL_KINDS
    }

    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        super::helpers::FIELD_METHODS
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if !is_field_call(source, node) {
            return;
        }
        let Some(class) = enclosing_class(node) else {
            return;
        };
        let Some(method) = find_method_def(class, source, &resolver_method_name(source, node))
        else {
            return;
        };
        let Some(key) = hash_key_from_method(source, method) else {
            return;
        };
        if is_conflict_field_name(&key) {
            return;
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        diagnostics.push(self.diagnostic(source, line, col, format!("Use hash_key: :{key}")));
    }
}

/// `def x; object[:key]; end` or `object["key"]`
fn hash_key_from_method(source: &SourceFile, method: Node<'_>) -> Option<String> {
    let expr = sole_body_expr(method)?;
    element_ref_key(source, expr).or_else(|| bracket_call_key(source, expr))
}

fn sole_body_expr(method: Node<'_>) -> Option<Node<'_>> {
    let body = method.child_by_field_name("body")?;
    let stmts = if body.kind() == "body_statement" {
        named_kids(body)
    } else {
        vec![body]
    };
    (stmts.len() == 1).then_some(stmts[0])
}

fn element_ref_key(source: &SourceFile, expr: Node<'_>) -> Option<String> {
    if expr.kind() != "element_reference" {
        return None;
    }
    let mut cur = expr.walk();
    let kids: Vec<_> = expr.named_children(&mut cur).collect();
    let recv = *kids.first()?;
    let key = *kids.get(1)?;
    call_is_object(source, recv)
        .then(|| sym_or_str(source, key))
        .flatten()
}

fn bracket_call_key(source: &SourceFile, expr: Node<'_>) -> Option<String> {
    if !matches!(expr.kind(), "call" | "command") {
        return None;
    }
    if call_method_name(source, expr) != Some(b"[]") {
        return None;
    }
    let recv = call_receiver(expr)?;
    if !call_is_object(source, recv) {
        return None;
    }
    argument_nodes(expr)
        .first()
        .and_then(|a| sym_or_str(source, *a))
}

fn call_is_object(source: &SourceFile, node: Node<'_>) -> bool {
    node_text(source, node) == "object"
}

fn sym_or_str(source: &SourceFile, node: Node<'_>) -> Option<String> {
    super::helpers::sym_text(source, node).or_else(|| {
        let t = node_text(source, node);
        if matches!(node.kind(), "string" | "string_content")
            || t.starts_with('"')
            || t.starts_with('\'')
        {
            Some(t.trim().trim_matches('"').trim_matches('\'').to_string())
        } else {
            None
        }
    })
}
