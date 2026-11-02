//! Prefer Rails-aware dates and conversions, preserving the original unsafe correction policy.
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, method_node, node_bytes, node_text,
    push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use std::sync::LazyLock;
use tree_sitter::Node;
pub struct Date;
static ZONE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?:[+-][0-9:]+|[0-9]Z)\z").unwrap());
fn safe_navigation(source: &SourceFile, node: Node<'_>) -> bool {
    node.child_by_field_name("operator")
        .is_some_and(|op| node_bytes(source, op) == b"&.")
}
fn chain(source: &SourceFile, node: Node<'_>) -> Vec<String> {
    let mut methods = Vec::new();
    let mut current = Some(node);
    while let Some(call) = current {
        if call.kind() == "call"
            && (call == node || !safe_navigation(source, call))
            && !call.child_by_field_name("block").is_some_and(|block| {
                node.start_byte() >= block.start_byte() && node.end_byte() <= block.end_byte()
            })
        {
            if let Some(name) = call_method_name(source, call) {
                let mut name = String::from_utf8_lossy(name).into_owned();
                if crate::cop::shared::is_setter_call(call) {
                    name.push('=');
                }
                methods.push(name);
            }
        }
        current = call.parent();
    }
    methods
}
fn date_name<'a>(source: &SourceFile, node: Node<'a>) -> Option<Node<'a>> {
    match node.kind() {
        "constant" if node_bytes(source, node) == b"Date" => Some(node),
        "scope_resolution" if node.child_by_field_name("scope").is_none() => node
            .child_by_field_name("name")
            .filter(|n| node_bytes(source, *n) == b"Date"),
        _ => None,
    }
}
impl Date {
    fn report_day(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        name: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: &mut Option<&mut Vec<Correction>>,
    ) {
        let methods = chain(source, node);
        let strict = config.get_str("EnforcedStyle", "flexible") == "strict";
        let mut bad = Vec::new();
        for method in methods {
            if (method == "today"
                || strict && matches!(method.as_str(), "current" | "yesterday" | "tomorrow"))
                && !bad.contains(&method)
            {
                bad.push(method);
            }
        }
        if bad.is_empty() {
            return;
        }
        let called = bad.join(".");
        let day = if called == "current" {
            "today"
        } else {
            called.as_str()
        };
        let Some(selector) = method_node(node) else {
            return;
        };
        let (line, col) = source.offset_to_line_col(selector.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            format!("Do not use `Date.{called}` without zone. Use `Time.zone.{day}` instead."),
        );
        d.highlight_length = node_text(source, selector).chars().count();
        d.correctable =
            !config.autocorrect_disabled() && call_method_name(source, node) != Some(b"current");
        if d.correctable {
            d.corrected = push_replace(
                corrections,
                name.start_byte(),
                name.end_byte(),
                "Time.zone",
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}
impl Cop for Date {
    fn name(&self) -> &'static str {
        "Rails/Date"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn safe_autocorrect(&self) -> bool {
        false
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "constant", "scope_resolution"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"to_time", b"to_time_in_current_zone"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if node.kind() != "call" {
            let Some(name) = date_name(source, node) else {
                return;
            };
            let Some(call) = node.parent().filter(|parent| {
                parent.kind() == "call"
                    && call_receiver(*parent) == Some(node)
                    && !safe_navigation(source, *parent)
            }) else {
                return;
            };
            self.report_day(source, call, name, config, diagnostics, &mut corrections);
            return;
        }
        if crate::cop::shared::is_setter_call(node) {
            return;
        }
        let Some(method @ (b"to_time" | b"to_time_in_current_zone")) =
            call_method_name(source, node)
        else {
            return;
        };
        let Some(receiver) = call_receiver(node) else {
            return;
        };
        if method == b"to_time" && config.get_bool("AllowToTime", true) {
            return;
        }
        if config.get_str("EnforcedStyle", "flexible") != "strict"
            && chain(source, node).iter().any(|name| {
                super::time_zone::ACCEPTED_METHODS
                    .iter()
                    .any(|method| name.as_bytes() == *method)
            })
        {
            return;
        }
        if method == b"to_time" {
            let literal = if matches!(
                receiver.kind(),
                "string" | "character" | "heredoc_beginning"
            ) {
                crate::cop::ported::ruby_string_parts(source, receiver)
                    .filter(|parts| parts.len() == 1)
            } else {
                None
            };
            if let Some(parts) = literal {
                if ZONE.is_match(&parts[0]) {
                    return;
                }
            } else if argument_nodes(node).len() == 1 {
                return;
            }
        }
        let Some(selector) = method_node(node) else {
            return;
        };
        let (line, col) = source.offset_to_line_col(selector.start_byte());
        let deprecated = method == b"to_time_in_current_zone";
        let message = if deprecated {
            "`to_time_in_current_zone` is deprecated. Use `in_time_zone` instead.".into()
        } else {
            "Do not use `to_time` on Date objects, because they know nothing about the time zone in use.".into()
        };
        let mut d = self.diagnostic(source, line, col, message);
        d.highlight_length = node_text(source, selector).chars().count();
        d.correctable = deprecated && !config.autocorrect_disabled();
        if d.correctable {
            d.corrected = push_replace(
                &mut corrections,
                selector.start_byte(),
                selector.end_byte(),
                "in_time_zone",
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Date, "cops/rails/date");
}
