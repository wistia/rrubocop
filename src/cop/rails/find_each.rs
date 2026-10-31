//! Rails/FindEach — `all.each` / `where.each` should be `find_each`.

use regex::Regex;
use tree_sitter::Node;

use crate::cop::shared::{
    call_method_name, call_receiver, method_node, node_bytes, node_text, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct FindEach;

const MSG: &str = "Use `find_each` instead of `each`.";

fn is_call(node: Node<'_>) -> bool {
    matches!(node.kind(), "call" | "command" | "command_call")
}

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
    call_method_name(source, recv) == Some(b"where")
        && call_receiver(recv).is_some_and(|r| is_errors(source, r))
}

/// Bare `errors` is an identifier in tree-sitter and a send in RuboCop.
fn is_errors(source: &SourceFile, node: Node<'_>) -> bool {
    if node.kind() == "identifier" {
        return node_bytes(source, node) == b"errors";
    }
    is_call(node) && call_method_name(source, node) == Some(b"errors")
}

fn scope_name<'a>(source: &'a SourceFile, recv: Node<'_>) -> Option<&'a [u8]> {
    let name = if is_call(recv) {
        call_method_name(source, recv)?
    } else if recv.kind() == "identifier" && node_bytes(source, recv) == b"all" {
        node_bytes(source, recv)
    } else {
        return None;
    };
    is_scope(name).then_some(name)
}

fn ar_class(source: &SourceFile, class_node: Node<'_>) -> bool {
    let Some(sc) = class_node.child_by_field_name("superclass") else {
        return false;
    };
    let mut cur = sc.walk();
    sc.named_children(&mut cur).next().is_some_and(|expr| {
        let name = node_text(source, expr);
        let name = name.trim().trim_start_matches("::");
        name == "ApplicationRecord" || name == "ActiveRecord::Base"
    })
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

fn listed(config: &CopConfig, key: &str, name: &[u8]) -> bool {
    config
        .options
        .get(key)
        .and_then(|v| v.as_sequence())
        .is_some_and(|seq| {
            seq.iter()
                .any(|v| v.as_str().is_some_and(|s| s.as_bytes() == name))
        })
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
    config
        .options
        .get("AllowedPatterns")
        .and_then(|v| v.as_sequence())
        .is_some_and(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str())
                .any(|pat| Regex::new(pat).is_ok_and(|re| re.is_match(text)))
        })
}

fn name_allowed(config: &CopConfig, name: &[u8]) -> bool {
    configured_allowed(config, name)
        || listed(config, "IgnoredMethods", name)
        || listed(config, "ExcludedMethods", name)
        || pattern_hit(config, name)
}

/// Receiver chain and arguments, not the block (`each { order }` still flags).
fn any_allowed(source: &SourceFile, node: Node<'_>, config: &CopConfig) -> bool {
    if is_call(node) {
        if call_method_name(source, node).is_some_and(|n| name_allowed(config, n)) {
            return true;
        }
        if call_receiver(node).is_some_and(|r| any_allowed(source, r, config)) {
            return true;
        }
        return node
            .child_by_field_name("arguments")
            .is_some_and(|args| any_allowed(source, args, config));
    }
    let mut cur = node.walk();
    node.named_children(&mut cur)
        .any(|child| any_allowed(source, child, config))
}

fn scope_each<'a>(source: &SourceFile, node: Node<'a>) -> Option<Node<'a>> {
    if call_method_name(source, node) != Some(b"each") {
        return None;
    }
    let recv = call_receiver(node)?;
    scope_name(source, recv).map(|_| recv)
}

fn report(
    cop: &FindEach,
    source: &SourceFile,
    node: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let meth = method_node(node).unwrap_or(node);
    let (line, col) = source.offset_to_line_col(meth.start_byte());
    let mut diag = cop.diagnostic(source, line, col, MSG.to_string());
    diag.highlight_length = meth.end_byte().saturating_sub(meth.start_byte()).max(1);
    if push_replace(
        corrections,
        meth.start_byte(),
        meth.end_byte(),
        "find_each",
        cop.name(),
    ) {
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
        &["call", "command", "command_call"]
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
        if (recv.kind() == "identifier" || call_receiver(recv).is_none())
            && !in_active_record(source, node)
        {
            return;
        }
        if errors_where(source, recv) || any_allowed(source, node, config) {
            return;
        }
        report(self, source, node, diagnostics, &mut corrections);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(FindEach, "cops/rails/find_each");
}
