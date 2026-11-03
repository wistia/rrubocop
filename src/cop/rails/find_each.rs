//! Rails/FindEach — `all.each` / `where.each` should be `find_each`.

use crate::cop::shared::ruby_expression::{regular_send_name, regular_send_receiver};
use regex::Regex;
use std::borrow::Cow;
use tree_sitter::Node;

use crate::cop::shared::{
    argument_nodes, call_receiver, method_node, named_kids, node_bytes, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct FindEach;

const MSG: &str = "Use `find_each` instead of `each`.";

fn is_scope(name: &[u8]) -> bool {
    matches!(
        name,
        b"all"
            | b"eager_load"
            | b"includes"
            | b"joins"
            | b"left_joins"
            | b"left_outer_joins"
            | b"not"
            | b"or"
            | b"preload"
            | b"references"
            | b"unscoped"
            | b"where"
    )
}
fn errors_where(source: &SourceFile, recv: Node<'_>) -> bool {
    regular_send_name(source, recv).as_deref() == Some(b"where")
        && call_receiver(recv).is_some_and(|errors| {
            errors.child_by_field_name("block").is_none()
                && regular_send_name(source, errors).as_deref() == Some(b"errors")
        })
}
fn scope_name<'a>(source: &'a SourceFile, recv: Node<'_>) -> Option<Cow<'a, [u8]>> {
    if recv.child_by_field_name("block").is_some() {
        return None;
    }
    let name = regular_send_name(source, recv)?;
    is_scope(&name).then_some(name)
}
fn root_constant(source: &SourceFile, node: Node<'_>, expected: &[u8]) -> bool {
    (node.kind() == "constant" && node_bytes(source, node) == expected)
        || (node.kind() == "scope_resolution"
            && node.child_by_field_name("scope").is_none()
            && node
                .child_by_field_name("name")
                .is_some_and(|name| node_bytes(source, name) == expected))
}
fn ar_class(source: &SourceFile, class_node: Node<'_>) -> bool {
    let Some(sc) = class_node
        .child_by_field_name("superclass")
        .and_then(|sc| sc.named_child(0))
    else {
        return false;
    };
    root_constant(source, sc, b"ApplicationRecord")
        || (sc.kind() == "scope_resolution"
            && sc
                .child_by_field_name("name")
                .is_some_and(|name| node_bytes(source, name) == b"Base")
            && sc
                .child_by_field_name("scope")
                .is_some_and(|scope| root_constant(source, scope, b"ActiveRecord")))
}

fn in_active_record(source: &SourceFile, node: Node<'_>) -> bool {
    let mut p = node.parent();
    while let Some(n) = p {
        if n.kind() == "class" && ar_class(source, n) {
            return true;
        }
        p = n.parent();
    }
    false
}

fn configured_strings<'a>(config: &'a CopConfig, key: &str) -> Vec<&'a str> {
    match config.options.get(key) {
        Some(serde_yml::Value::Sequence(values)) => {
            values.iter().filter_map(|value| value.as_str()).collect()
        }
        Some(serde_yml::Value::String(value)) => vec![value.as_str()],
        _ => vec![],
    }
}
fn listed(config: &CopConfig, key: &str, name: &[u8]) -> bool {
    configured_strings(config, key)
        .iter()
        .any(|value| value.as_bytes() == name)
}

fn configured_allowed(config: &CopConfig, name: &[u8]) -> bool {
    if config.options.contains_key("AllowedMethods") {
        return listed(config, "AllowedMethods", name);
    }
    matches!(name, b"order" | b"limit" | b"select" | b"lock")
}

fn pattern_hit(config: &CopConfig, name: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(name) else {
        return false;
    };
    ["AllowedPatterns", "IgnoredPatterns"]
        .into_iter()
        .flat_map(|key| configured_strings(config, key))
        .any(|pattern| Regex::new(pattern).is_ok_and(|re| re.is_match(text)))
}

fn name_allowed(config: &CopConfig, name: &[u8]) -> bool {
    configured_allowed(config, name)
        || listed(config, "IgnoredMethods", name)
        || listed(config, "ExcludedMethods", name)
        || pattern_hit(config, name)
}

/// Receiver chain and arguments, not the block (`each { order }` still flags).
fn any_allowed(source: &SourceFile, node: Node<'_>, config: &CopConfig) -> bool {
    fn walk(source: &SourceFile, node: Node<'_>, config: &CopConfig, own_block: bool) -> bool {
        if regular_send_name(source, node).is_some_and(|name| name_allowed(config, &name)) {
            return true;
        }
        let excluded = own_block
            .then(|| node.child_by_field_name("block"))
            .flatten();
        let mut cursor = node.walk();
        node.named_children(&mut cursor)
            .filter(|child| Some(*child) != excluded)
            .any(|child| walk(source, child, config, false))
    }
    walk(source, node, config, true)
}

fn scope_each<'a>(source: &SourceFile, node: Node<'a>) -> Option<Node<'a>> {
    if regular_send_name(source, node).as_deref() != Some(b"each") {
        return None;
    }
    let recv = call_receiver(node)?;
    scope_name(source, recv).map(|_| recv)
}

struct RelationOverrides(bool);
fn overridden(source: &SourceFile, node: Node<'_>) -> bool {
    source
        .cached(0, || {
            let mut root = node;
            while let Some(parent) = root.parent() {
                root = parent;
            }
            let mut pending = vec![root];
            while let Some(node) = pending.pop() {
                if matches!(node.kind(), "method" | "singleton_method")
                    && node.child_by_field_name("name").is_some_and(|name| {
                        is_scope(node_bytes(source, name))
                            || matches!(node_bytes(source, name), b"each" | b"find_each")
                    })
                {
                    return RelationOverrides(true);
                }
                if (node.kind() == "assignment"
                    && node
                        .child_by_field_name("left")
                        .is_some_and(|left| matches!(left.kind(), "constant" | "scope_resolution")))
                    || node.kind() == "alias"
                    || regular_send_name(source, node).is_some_and(|name| {
                        matches!(
                            name.as_ref(),
                            b"alias_method"
                                | b"define_method"
                                | b"define_singleton_method"
                                | b"class_eval"
                                | b"module_eval"
                                | b"eval"
                                | b"prepend"
                        )
                    })
                {
                    return RelationOverrides(true);
                }
                pending.extend(named_kids(node));
            }
            RelationOverrides(false)
        })
        .0
}

fn report(
    cop: &FindEach,
    source: &SourceFile,
    node: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
    config: &CopConfig,
) {
    let meth = method_node(node).unwrap_or(node);
    let (line, col) = source.offset_to_line_col(meth.start_byte());
    let mut diag = cop.diagnostic(source, line, col, MSG.to_string());
    diag.highlight_length = meth.end_byte().saturating_sub(meth.start_byte()).max(1);
    diag.correctable = !config.autocorrect_disabled()
        && !crate::cop::shared::ruby_semantics::value_used(source, node)
        && node.child_by_field_name("block").is_some()
        && argument_nodes(node).is_empty()
        && !overridden(source, node);
    if diag.correctable
        && push_replace(
            corrections,
            meth.start_byte(),
            meth.end_byte(),
            "find_each",
            cop.name(),
        )
    {
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

impl Cop for FindEach {
    fn name(&self) -> &'static str {
        "Rails/FindEach"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn safe_autocorrect(&self) -> bool {
        false
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }

    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"each"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(recv) = scope_each(source, node) else {
            return;
        };
        if regular_send_receiver(source, recv) == Some(None) && !in_active_record(source, node) {
            return;
        }
        if errors_where(source, recv) || any_allowed(source, node, config) {
            return;
        }
        report(self, source, node, diagnostics, &mut corrections, config);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(FindEach, "cops/rails/find_each");
}
