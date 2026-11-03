//! GraphQL/ExtractType — fields sharing a prefix suggest a nested type.

use std::collections::HashMap;

use tree_sitter::Node;

use super::helpers::{
    DEPT_INCLUDE, class_body_stmts, field_name, is_field_call, nested_class, underscore,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ExtractType;

impl Cop for ExtractType {
    fn name(&self) -> &'static str {
        "GraphQL/ExtractType"
    }

    fn default_include(&self) -> &'static [&'static str] {
        DEPT_INCLUDE
    }

    fn default_exclude(&self) -> &'static [&'static str] {
        &["**/graphql/mutations/**/*.rb"]
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["class"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if nested_class(node) {
            return;
        }
        let max_fields = config.get_usize("MaxFields", 2);
        let mut sorted: Vec<_> =
            prefix_groups(&underscored_fields(source, node), &ignored_prefixes(config))
                .into_iter()
                .collect();
        sorted.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        emit_groups(self, source, sorted, max_fields, diagnostics);
    }
}

fn ignored_prefixes(config: &CopConfig) -> Vec<String> {
    let list = super::helpers::config_string_list(config, "Prefixes");
    if list.is_empty() {
        ["is", "has", "with", "avg", "min", "max"]
            .into_iter()
            .map(String::from)
            .collect()
    } else {
        list
    }
}

fn underscored_fields<'a>(source: &SourceFile, node: Node<'a>) -> Vec<(String, Node<'a>)> {
    class_body_stmts(node)
        .into_iter()
        .filter(|n| is_field_call(source, *n))
        .filter_map(|n| field_name(source, n).map(|name| (underscore(&name), n)))
        .filter(|(name, _)| name.contains('_'))
        .collect()
}

fn emit_groups(
    cop: &ExtractType,
    source: &SourceFile,
    sorted: Vec<(String, Vec<(String, Node<'_>)>)>,
    max_fields: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut already = std::collections::HashSet::new();
    for (prefix, mut group) in sorted {
        group.retain(|(n, _)| !already.contains(n));
        if group.len() < max_fields {
            continue;
        }
        already.extend(push_group(cop, source, &prefix, &group, diagnostics));
    }
}

fn push_group(
    cop: &ExtractType,
    source: &SourceFile,
    prefix: &str,
    group: &[(String, Node<'_>)],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<String> {
    let names: Vec<_> = group.iter().map(|(n, _)| n.as_str()).collect();
    let (line, col) = source.offset_to_line_col(group.last().unwrap().1.start_byte());
    diagnostics.push(cop.diagnostic(
        source,
        line,
        col,
        format!(
            "Consider moving {} to a new type and adding the `{prefix}` field instead",
            names.join(", ")
        ),
    ));
    group.iter().map(|(n, _)| n.clone()).collect()
}

fn prefix_groups<'a>(
    fields: &[(String, Node<'a>)],
    ignore: &[String],
) -> HashMap<String, Vec<(String, Node<'a>)>> {
    let mut by_prefix = HashMap::new();
    for (uname, n) in fields {
        add_prefixes(&mut by_prefix, uname, *n, ignore);
    }
    by_prefix
}

fn add_prefixes<'a>(
    by_prefix: &mut HashMap<String, Vec<(String, Node<'a>)>>,
    uname: &str,
    n: Node<'a>,
    ignore: &[String],
) {
    let parts: Vec<&str> = uname.split('_').collect();
    let mut prev = Vec::new();
    for part in &parts[..parts.len().saturating_sub(1)] {
        prev.push(*part);
        let prefix = prev.join("_");
        if ignore.iter().any(|p| p == &prefix) {
            break;
        }
        by_prefix
            .entry(prefix)
            .or_default()
            .push((uname.to_string(), n));
    }
}
