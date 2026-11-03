//! RSpecRails/InferredSpecType — redundant `type:` metadata from path.

use tree_sitter::Node;

use crate::cop::rspec::helpers::{bare_rspec_call, call_block};
use crate::cop::shared::{argument_nodes, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct InferredSpecType;

const DEFAULT_INFERENCES: &[(&str, &str)] = &[
    ("channels", "channel"),
    ("controllers", "controller"),
    ("features", "feature"),
    ("generator", "generator"),
    ("helpers", "helper"),
    ("jobs", "job"),
    ("mailboxes", "mailbox"),
    ("mailers", "mailer"),
    ("models", "model"),
    ("requests", "request"),
    ("integration", "request"),
    ("api", "request"),
    ("routing", "routing"),
    ("system", "system"),
    ("views", "view"),
];

const GROUPS: &[&[u8]] = &[
    b"describe",
    b"context",
    b"feature",
    b"example_group",
    b"xdescribe",
    b"xcontext",
    b"xfeature",
    b"xexample_group",
];

fn infer_type(path: &str, config: &CopConfig) -> Option<String> {
    if let Some(serde_yml::Value::Mapping(map)) = config.options.get("Inferences") {
        for (k, v) in map {
            let (Some(prefix), Some(inf)) = (k.as_str(), v.as_str()) else {
                continue;
            };
            if path.contains(&format!("spec/{prefix}/")) {
                return Some(inf.to_string());
            }
        }
        return None;
    }
    DEFAULT_INFERENCES
        .iter()
        .find(|(prefix, _)| path.contains(&format!("spec/{prefix}/")))
        .map(|(_, inf)| (*inf).to_string())
}

fn pair_key_is_type(source: &SourceFile, pair: Node<'_>) -> bool {
    pair.child_by_field_name("key")
        .is_some_and(|key| matches!(node_bytes(source, key), b"type" | b":type" | b"type:"))
}

fn pair_type_value<'a>(source: &'a SourceFile, pair: Node<'_>) -> Option<&'a str> {
    std::str::from_utf8(node_bytes(source, pair.child_by_field_name("value")?))
        .ok()
        .map(|s| s.trim_start_matches(':'))
}

fn find_type_pair<'a>(source: &SourceFile, node: Node<'a>) -> Option<(Node<'a>, usize)> {
    match node.kind() {
        "pair" if pair_key_is_type(source, node) => Some((node, 1)),
        "hash" | "bare_hash" => {
            let mut cur = node.walk();
            let pairs: Vec<_> = node
                .named_children(&mut cur)
                .filter(|c| c.kind() == "pair")
                .collect();
            let count = pairs.len();
            pairs
                .into_iter()
                .find(|p| pair_key_is_type(source, *p))
                .map(|p| (if count == 1 { node } else { p }, count))
        }
        _ => None,
    }
}

fn type_value<'a>(source: &'a SourceFile, off_node: Node<'_>) -> Option<&'a str> {
    if off_node.kind() == "pair" {
        return pair_type_value(source, off_node);
    }
    let mut cur = off_node.walk();
    off_node
        .named_children(&mut cur)
        .find(|c| c.kind() == "pair")
        .and_then(|p| pair_type_value(source, p))
}

fn is_positional(arg: Node<'_>) -> bool {
    !matches!(arg.kind(), "pair" | "hash" | "bare_hash")
}

fn scan_type_args<'a>(
    source: &'a SourceFile,
    node: Node<'a>,
    inferred: &str,
) -> Option<(Node<'a>, bool)> {
    let mut positional_before = false;
    for arg in argument_nodes(node) {
        if let Some((off_node, pair_count)) = find_type_pair(source, arg) {
            let val = type_value(source, off_node)?;
            if val != inferred {
                continue;
            }
            let skip = pair_count == 1 && !positional_before;
            return Some((off_node, skip));
        }
        if is_positional(arg) {
            positional_before = true;
        }
    }
    None
}

impl Cop for InferredSpecType {
    fn name(&self) -> &'static str {
        "RSpecRails/InferredSpecType"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn safe_autocorrect(&self) -> bool {
        false
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*_spec.rb", "**/spec/**/*"]
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "command"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let Some(method) = bare_rspec_call(source, node) else {
            return;
        };
        if !GROUPS.iter().any(|&g| g == method) || call_block(node).is_none() {
            return;
        }
        #[derive(Clone)]
        struct Inferred(Option<String>);
        let Inferred(Some(inferred)) =
            crate::cop::rspec::cache::config_fact(source, config, &["Inferences"], || {
                Inferred(infer_type(source.path_str(), config))
            })
        else {
            return;
        };
        let Some((off_node, skip)) = scan_type_args(source, node, &inferred) else {
            return;
        };
        if skip {
            return;
        }
        let (line, col) = source.offset_to_line_col(off_node.start_byte());
        diagnostics.push(self.diagnostic(source, line, col, "Remove redundant spec type.".into()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InferredSpecType, "cops/rspec_rails/inferred_spec_type");
}
