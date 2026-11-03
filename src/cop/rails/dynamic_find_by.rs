//! Prefer explicit Active Record finder keyword arguments.
use crate::cop::shared::{
    argument_nodes, call_receiver, method_node, named_kids, node_text, push_replace, send_end,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct DynamicFindBy;
fn listed(config: &CopConfig, key: &str, name: &str, defaults: &[&str]) -> bool {
    if let Some(value) = config.options.get(key) {
        value
            .as_sequence()
            .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(name)))
    } else {
        defaults.contains(&name)
    }
}
fn constant_path(source: &SourceFile, node: Node<'_>) -> Option<String> {
    match node.kind() {
        "constant" => Some(node_text(source, node)),
        "scope_resolution" => {
            let name = node.child_by_field_name("name")?;
            let scope = match node.child_by_field_name("scope") {
                Some(n) => constant_path(source, n)?,
                None => String::new(),
            };
            Some(format!("{}::{}", scope, node_text(source, name)))
        }
        _ => None,
    }
}
fn model(source: &SourceFile, node: Node<'_>) -> bool {
    let mut parent = node.parent();
    while let Some(n) = parent {
        if n.kind() == "class"
            && n.child_by_field_name("superclass")
                .and_then(|n| named_kids(n).first().copied())
                .and_then(|n| constant_path(source, n))
                .is_some_and(|s| {
                    matches!(
                        s.as_str(),
                        "ApplicationRecord"
                            | "::ApplicationRecord"
                            | "ActiveRecord::Base"
                            | "::ActiveRecord::Base"
                    )
                })
        {
            return true;
        }
        parent = n.parent();
    }
    false
}
impl Cop for DynamicFindBy {
    fn name(&self) -> &'static str {
        "Rails/DynamicFindBy"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn safe_autocorrect(&self) -> bool {
        false
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "identifier"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if crate::cop::shared::is_setter_call(node) {
            return;
        }
        let Some(method) = method_node(node)
            .or_else(|| crate::cop::ported::bare_call_identifier(source, node).then_some(node))
        else {
            return;
        };
        let name = node_text(source, method);
        let Some(columns) = name.strip_prefix("find_by_") else {
            return;
        };
        let (columns, bang) = columns
            .strip_suffix('!')
            .map_or((columns, false), |s| (s, true));
        if columns.is_empty()
            || listed(
                config,
                "AllowedMethods",
                &name,
                &["find_by_sql", "find_by_token_for"],
            )
            || listed(
                config,
                "Whitelist",
                &name,
                &["find_by_sql", "find_by_token_for"],
            )
        {
            return;
        }
        let receiver = call_receiver(node);
        if receiver.is_none() && !model(source, node)
            || receiver.is_some_and(|r| {
                listed(
                    config,
                    "AllowedReceivers",
                    &node_text(source, r),
                    &["Gem::Specification", "page"],
                )
            })
        {
            return;
        }
        let args = argument_nodes(node);
        let mut keys: Vec<_> = columns.split("_and_").collect();
        while keys.last() == Some(&"") {
            keys.pop();
        }
        if keys.len() != args.len()
            || args.iter().any(|a| {
                matches!(
                    a.kind(),
                    "hash" | "pair" | "hash_splat_argument" | "splat_argument"
                )
            })
        {
            return;
        }
        let replacement = if bang { "find_by!" } else { "find_by" };
        let end = send_end(node);
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            format!("Use `{replacement}` instead of dynamic `{name}`."),
        );
        d.highlight_length = super::blank::highlight_length(source, node.start_byte(), end);
        let mut proposed = source.as_bytes().to_vec();
        for (arg, key) in args.iter().zip(&keys).rev() {
            proposed.splice(
                arg.start_byte()..arg.start_byte(),
                format!("{key}: ").bytes(),
            );
        }
        proposed.splice(method.start_byte()..method.end_byte(), replacement.bytes());
        let proposed = SourceFile::from_bytes("dynamic_find_by.rb", proposed);
        let unique = keys
            .iter()
            .enumerate()
            .all(|(i, key)| !keys[..i].contains(key));
        d.correctable = unique
            && !config.autocorrect_disabled()
            && crate::parse::parse_ruby(&proposed).is_ok_and(|t| !t.root_node().has_error());
        if d.correctable {
            d.corrected = push_replace(
                &mut corrections,
                method.start_byte(),
                method.end_byte(),
                replacement,
                self.name(),
            );
            for (arg, key) in args.iter().zip(keys) {
                push_replace(
                    &mut corrections,
                    arg.start_byte(),
                    arg.start_byte(),
                    format!("{key}: "),
                    self.name(),
                );
            }
        }
        diagnostics.push(d);
    }
}
