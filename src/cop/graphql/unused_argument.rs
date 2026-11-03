//! GraphQL/UnusedArgument — resolve/authorized? must list declared arguments.

use tree_sitter::Node;

use super::helpers::{
    DEPT_INCLUDE, class_body_stmts, effective_arg_name, is_argument_call, nested_class,
};
use crate::cop::shared::{named_kids, node_text};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct UnusedArgument;

fn first_resolution_method(source: &SourceFile, node: Node<'_>) -> Option<String> {
    class_body_stmts(node).into_iter().find_map(|n| {
        let name = n.child_by_field_name("name")?;
        let text = node_text(source, name);
        matches!(text.as_str(), "resolve" | "authorized?").then_some(text)
    })
}

impl Cop for UnusedArgument {
    fn name(&self) -> &'static str {
        "GraphQL/UnusedArgument"
    }

    fn default_include(&self) -> &'static [&'static str] {
        DEPT_INCLUDE
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["class"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if nested_class(node) {
            return;
        }
        let declared = collect_declared_args(source, node);
        if declared.is_empty() {
            return;
        }
        let first = first_resolution_method(source, node);
        for method_name in first.as_deref().into_iter() {
            let Some((method, msg)) = missing_for(source, node, method_name, &declared) else {
                continue;
            };
            let (line, col) = source.offset_to_line_col(method.start_byte());
            diagnostics.push(self.diagnostic(source, line, col, msg));
        }
    }
}

fn missing_for<'a>(
    source: &SourceFile,
    class_node: Node<'a>,
    method_name: &str,
    declared: &[String],
) -> Option<(Node<'a>, String)> {
    let method = find_instance_method(source, class_node, method_name)?;
    if ignore_args(method) {
        return None;
    }
    let msg = format_missing(method_name, &missing_args(source, method, declared))?;
    Some((method, msg))
}

fn missing_args(source: &SourceFile, method: Node<'_>, declared: &[String]) -> Vec<String> {
    let kwargs: std::collections::HashSet<_> =
        method_kwarg_names(source, method).into_iter().collect();
    let mut missing: Vec<_> = declared
        .iter()
        .filter(|a| !kwargs.contains(*a))
        .cloned()
        .collect();
    missing.sort();
    missing.dedup();
    missing
}

fn format_missing(method_name: &str, missing: &[String]) -> Option<String> {
    if missing.is_empty() {
        return None;
    }
    let listed = missing
        .iter()
        .map(|a| format!("{a}:"))
        .collect::<Vec<_>>()
        .join(", ");
    let ending = if missing.len() == 1 { "" } else { "s" };
    Some(format!(
        "Argument{ending} `{listed}` should be listed in the {method_name} signature."
    ))
}

fn collect_declared_args(source: &SourceFile, class_node: Node<'_>) -> Vec<String> {
    let mut out = Vec::new();
    walk_args(source, class_node, class_node, &mut out);
    out
}

fn walk_args(source: &SourceFile, node: Node<'_>, class_node: Node<'_>, out: &mut Vec<String>) {
    if is_skipped_scope(node, class_node) {
        return;
    }
    if is_argument_call(source, node) && under_class(node, class_node) {
        if let Some(n) = effective_arg_name(source, node) {
            out.push(n);
        }
    }
    let mut cur = node.walk();
    for child in node.children(&mut cur) {
        if child.kind() == "class" && child.id() != class_node.id() {
            continue;
        }
        if matches!(child.kind(), "method" | "singleton_method" | "lambda") {
            continue;
        }
        walk_args(source, child, class_node, out);
    }
}

fn is_skipped_scope(node: Node<'_>, class_node: Node<'_>) -> bool {
    node.id() != class_node.id()
        && matches!(
            node.kind(),
            "method" | "singleton_method" | "class" | "module" | "lambda"
        )
}

fn under_class(node: Node<'_>, class_node: Node<'_>) -> bool {
    let mut p = node.parent();
    while let Some(x) = p {
        if x.kind() == "class" {
            return x.id() == class_node.id();
        }
        if matches!(x.kind(), "method" | "module" | "lambda") {
            return false;
        }
        p = x.parent();
    }
    false
}

fn find_instance_method<'a>(
    source: &SourceFile,
    class_node: Node<'a>,
    name: &str,
) -> Option<Node<'a>> {
    class_body_stmts(class_node).into_iter().find(|stmt| {
        stmt.kind() == "method"
            && stmt
                .child_by_field_name("name")
                .map(|n| node_text(source, n) == name)
                .unwrap_or(false)
    })
}

fn ignore_args(method: Node<'_>) -> bool {
    let Some(params) = method.child_by_field_name("parameters") else {
        return false;
    };
    named_kids(params).iter().any(|k| {
        matches!(
            k.kind(),
            "identifier"
                | "optional_parameter"
                | "splat_parameter"
                | "forward_argument"
                | "hash_splat_nil"
                | "hash_splat_parameter"
        )
    })
}

fn method_kwarg_names(source: &SourceFile, method: Node<'_>) -> Vec<String> {
    let Some(params) = method.child_by_field_name("parameters") else {
        return Vec::new();
    };
    named_kids(params)
        .into_iter()
        .filter_map(|p| kwarg_param_name(source, p))
        .collect()
}

fn kwarg_param_name(source: &SourceFile, p: Node<'_>) -> Option<String> {
    if !matches!(p.kind(), "keyword_parameter" | "optional_keyword_parameter") {
        return None;
    }
    if let Some(name) = p.child_by_field_name("name") {
        return Some(node_text(source, name));
    }
    let t = node_text(source, p);
    let name = t.split(':').next().unwrap_or(&t).trim();
    (!name.is_empty()).then(|| name.to_string())
}
