//! RSpec/MultipleMemoizedHelpers — too many unique `let`/`subject` names.

use std::collections::HashSet;

use tree_sitter::Node;

use crate::cop::rspec::helpers::{
    RSPEC_INCLUDE, bare_rspec_call, block_body, call_block, first_sym_arg, is_example, is_group,
};
use crate::cop::shared::{call_method_name, call_receiver, method_node, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MultipleMemoizedHelpers;

fn is_let(name: &[u8]) -> bool {
    name == b"let" || name == b"let!"
}

fn is_subject(name: &[u8]) -> bool {
    name == b"subject" || name == b"subject!"
}

fn is_include(name: &[u8]) -> bool {
    matches!(
        name,
        b"it_behaves_like" | b"it_should_behave_like" | b"include_examples" | b"include_context"
    )
}

fn is_rspec_bare(source: &SourceFile, node: Node<'_>) -> bool {
    match call_receiver(node) {
        None => true,
        Some(r) => r.kind() == "constant" && node_bytes(source, r) == b"RSpec",
    }
}

fn helper_name(source: &SourceFile, call: Node<'_>, method: &[u8]) -> Option<Vec<u8>> {
    if is_subject(method) {
        return Some(first_sym_arg(source, call).unwrap_or(b"subject").to_vec());
    }
    if is_let(method) {
        return first_sym_arg(source, call).map(|n| n.to_vec());
    }
    None
}

fn should_count(method: &[u8], allow_subject: bool) -> bool {
    is_let(method) || (!allow_subject && is_subject(method))
}

fn scope_boundary(method: &[u8]) -> bool {
    is_group(method) || is_include(method) || is_example(method)
}

fn record_helper(source: &SourceFile, node: Node<'_>, method: &[u8], out: &mut HashSet<Vec<u8>>) {
    match helper_name(source, node, method) {
        Some(name) => {
            out.insert(name);
        }
        None if is_let(method) => {
            out.insert(b"__unknown_variable__".to_vec());
        }
        None => {}
    }
}

fn walk_names(
    source: &SourceFile,
    node: Node<'_>,
    allow_subject: bool,
    out: &mut HashSet<Vec<u8>>,
) {
    if matches!(node.kind(), "call" | "command") {
        let method = call_method_name(source, node);
        if is_rspec_bare(source, node) && call_block(node).is_some() {
            if let Some(m) = method {
                if scope_boundary(m) {
                    return;
                }
                if should_count(m, allow_subject) {
                    record_helper(source, node, m, out);
                }
            }
        }
    }
    let mut cur = node.walk();
    for child in node.children(&mut cur) {
        walk_names(source, child, allow_subject, out);
    }
}

#[derive(Clone)]
struct NamesWithoutSubject(std::sync::Arc<HashSet<Vec<u8>>>);
#[derive(Clone)]
struct NamesWithSubject(std::sync::Arc<HashSet<Vec<u8>>>);
fn own_names(
    source: &SourceFile,
    body: Node<'_>,
    allow_subject: bool,
) -> std::sync::Arc<HashSet<Vec<u8>>> {
    let build = || {
        let mut names = HashSet::new();
        walk_names(source, body, allow_subject, &mut names);
        std::sync::Arc::new(names)
    };
    if allow_subject {
        super::cache::node_fact(source, body, || NamesWithoutSubject(build())).0
    } else {
        super::cache::node_fact(source, body, || NamesWithSubject(build())).0
    }
}

fn ancestor_names(
    source: &SourceFile,
    mut node: Node<'_>,
    allow_subject: bool,
    out: &mut HashSet<Vec<u8>>,
) {
    while let Some(parent) = node.parent() {
        node = parent;
        if !matches!(node.kind(), "call" | "command") {
            continue;
        }
        let Some(method) = bare_rspec_call(source, node) else {
            continue;
        };
        if !is_group(method) {
            continue;
        }
        if let Some(body) = call_block(node).and_then(block_body) {
            out.extend(own_names(source, body, allow_subject).iter().cloned());
        }
    }
}

fn report_too_many(
    cop: &MultipleMemoizedHelpers,
    source: &SourceFile,
    node: Node<'_>,
    total: usize,
    max: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let (line, col) = source.offset_to_line_col(method_node(node).unwrap_or(node).start_byte());
    diagnostics.push(cop.diagnostic(
        source,
        line,
        col,
        format!("Example group has too many memoized helpers [{total}/{max}]"),
    ));
}

fn group_helper_total(
    source: &SourceFile,
    node: Node<'_>,
    body: Node<'_>,
    allow_subject: bool,
) -> usize {
    let mut names = HashSet::new();
    names.extend(own_names(source, body, allow_subject).iter().cloned());
    ancestor_names(source, node, allow_subject, &mut names);
    names.len()
}

impl Cop for MultipleMemoizedHelpers {
    fn name(&self) -> &'static str {
        "RSpec/MultipleMemoizedHelpers"
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
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
        if !is_group(method) {
            return;
        }
        let Some(body) = call_block(node).and_then(block_body) else {
            return;
        };
        let max = config.get_usize("Max", 5);
        let total = group_helper_total(source, node, body, config.get_bool("AllowSubject", true));
        if total > max {
            report_too_many(self, source, node, total, max, diagnostics);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        MultipleMemoizedHelpers,
        "cops/rspec/multiple_memoized_helpers"
    );
}
