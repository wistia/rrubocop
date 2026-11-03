use crate::cop::rspec::helpers::bare_rspec_call;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::shared::node_text;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use regex::Regex;
use std::sync::OnceLock;
use tree_sitter::Node;

pub struct SpecFilePathFormat;
fn constant_name(source: &SourceFile, node: Node<'_>) -> Option<String> {
    match node.kind() {
        "constant" => Some(node_text(source, node)),
        "scope_resolution" => {
            let name = node.child_by_field_name("name")?;
            Some(if let Some(scope) = node.child_by_field_name("scope") {
                format!(
                    "{}::{}",
                    constant_name(source, scope).unwrap_or_default(),
                    node_text(source, name)
                )
            } else {
                node_text(source, name)
            })
        }
        _ => None,
    }
}
fn snake_case(name: &str) -> String {
    static LOWER: OnceLock<Regex> = OnceLock::new();
    static ACRONYM: OnceLock<Regex> = OnceLock::new();
    let lower = LOWER
        .get_or_init(|| Regex::new("([^A-Z])([A-Z]+)").unwrap())
        .replace_all(name, "${1}_${2}");
    ACRONYM
        .get_or_init(|| Regex::new("([A-Z])([A-Z][^A-Z0-9]+)").unwrap())
        .replace_all(&lower, "${1}_${2}")
        .to_lowercase()
}
fn ignored_metadata(source: &SourceFile, node: Node<'_>, config: &CopConfig) -> bool {
    let ignore = config
        .options
        .get("IgnoreMetadata")
        .and_then(|value| value.as_mapping());
    let mut pending: Vec<_> = call_args(node).into_iter().skip(1).collect();
    while let Some(child) = pending.pop() {
        if child.kind() == "pair" {
            if let (Some(key), Some(value)) = (
                child.child_by_field_name("key"),
                child.child_by_field_name("value"),
            ) {
                if symbol_node(key) && symbol_node(value) {
                    if let (Some(key), Some(value)) = (
                        crate::cop::ported::static_symbol(source, key),
                        crate::cop::ported::static_symbol(source, value),
                    ) {
                        if ignore
                            .and_then(|ignore| ignore.get(serde_yml::Value::String(key.clone())))
                            .and_then(|ignored| ignored.as_str())
                            == Some(value.as_str())
                            || ignore.is_none() && key == "type" && value == "routing"
                        {
                            return true;
                        }
                    }
                }
            }
        }
        pending.extend(code_children(child));
    }
    false
}
impl Cop for SpecFilePathFormat {
    fn name(&self) -> &'static str {
        "RSpec/SpecFilePathFormat"
    }
    fn default_include(&self) -> &'static [&'static str] {
        &["**/*_spec.rb"]
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["program"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let groups = top_groups(source, node);
        if groups.len() != 1 {
            return;
        }
        let group = groups[0];
        if numbered_block(source, group)
            || !bare_rspec_call(source, group).is_some_and(is_example_group)
            || ignored_metadata(source, group, config)
        {
            return;
        }
        let args = call_args(group);
        let Some(constant) = args
            .first()
            .copied()
            .and_then(|node| constant_name(source, node))
        else {
            return;
        };
        let mut namespaces = Vec::new();
        let mut ancestor = group.parent();
        while let Some(parent) = ancestor {
            if matches!(parent.kind(), "class" | "module") {
                if let Some(name) = parent
                    .child_by_field_name("name")
                    .and_then(|name| constant_name(source, name))
                {
                    namespaces.push(name);
                }
            }
            ancestor = parent.parent();
        }
        namespaces.reverse();
        namespaces.push(constant);
        let transform = config
            .options
            .get("CustomTransform")
            .and_then(|value| value.as_mapping());
        let expected = namespaces
            .iter()
            .flat_map(|name| name.split("::"))
            .map(|name| {
                transform
                    .and_then(|mapping| mapping.get(serde_yml::Value::String(name.to_string())))
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
                    .unwrap_or_else(|| snake_case(name))
            })
            .collect::<Vec<_>>()
            .join("/");
        let method = if config.get_bool("IgnoreMethods", false) {
            None
        } else {
            args.get(1)
                .filter(|method| matches!(method.kind(), "string" | "chained_string"))
                .and_then(|method| crate::cop::ported::static_literal(source, *method))
        };
        static SPACE: OnceLock<Regex> = OnceLock::new();
        static NON_WORD: OnceLock<Regex> = OnceLock::new();
        let pattern = if let Some(method) = method {
            let method = SPACE
                .get_or_init(|| Regex::new(r"[ \t\r\n\x0B\x0C]").unwrap())
                .replace_all(&method, "_");
            let method = NON_WORD
                .get_or_init(|| Regex::new(r"[^A-Za-z0-9_]").unwrap())
                .replace_all(&method, "");
            format!("{expected}.*{method}[^/]*_spec\\.rb")
        } else {
            format!("{expected}[^/]*_spec\\.rb")
        };
        if Regex::new(&format!("{pattern}$"))
            .ok()
            .is_some_and(|pattern| pattern.is_match(&source.path.to_string_lossy()))
        {
            return;
        }
        let suffix = pattern
            .replacen(".*", "*", 1)
            .replacen("[^/]*", "*", 1)
            .replacen("\\.", ".", 1);
        report(
            self,
            source,
            group.start_byte(),
            header_end(group),
            format!("Spec path should end with `{suffix}`."),
            diagnostics,
        );
    }
}
