use super::super::context::*;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::model::{Entry, FileModel, IntroKind, ScopeData, ScopeKind, WriteKind};
mod references;
use crate::parse::source::SourceFile;
use references::{referenced_before, uses};
use tree_sitter::Node;
pub(super) struct ShadowedArgument;
impl Cop for ShadowedArgument {
    fn name(&self) -> &'static str {
        "Lint/ShadowedArgument"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }
    fn needs_file_model(&self) -> bool {
        true
    }
    fn check_file_model(
        &self,
        source: &SourceFile,
        model: &FileModel<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let index = std::sync::OnceLock::new();
        for (scope_index, scope) in model.scopes.iter().enumerate() {
            if !matches!(scope.kind, ScopeKind::Method | ScopeKind::Block) {
                continue;
            }
            for (name, entry) in &scope.entries {
                if let Some(n) = shadow(model, scope_index, name, entry, config, &index) {
                    diagnostics.push(shadow_diagnostic(self, source, n, name));
                }
            }
        }
    }
}
fn shadow_diagnostic(
    cop: &ShadowedArgument,
    source: &SourceFile,
    n: Node<'_>,
    name: &str,
) -> Diagnostic {
    let (line, col) = source.offset_to_line_col(n.start_byte());
    let mut diagnostic = cop.diagnostic(
        source,
        line,
        col,
        format!("Argument `{name}` was shadowed by a local variable before it was used."),
    );
    diagnostic.highlight_length = n.end_byte() - n.start_byte();
    diagnostic
}
fn shadow<'n>(
    model: &'n FileModel<'_>,
    scope_index: usize,
    name: &str,
    entry: &Entry,
    config: &CopConfig,
    index: &std::sync::OnceLock<references::Index>,
) -> Option<Node<'n>> {
    let scope = &model.scopes[scope_index];
    let declaration = written_parameter(model, scope, entry)?;
    let index = index.get_or_init(|| references::Index::new(model));
    let reads = argument_reads(model, scope_index, name, entry, index)?;
    let (target, known, plain) = candidate(model, scope, name, entry, index)?;
    if referenced_before(
        model,
        index,
        entry,
        &reads,
        assignment(target).start_byte(),
        config,
    ) {
        return None;
    }
    Some(if !known {
        declaration
    } else if plain {
        assignment(target)
    } else {
        target
    })
}
fn written_parameter<'n>(
    model: &'n FileModel<'_>,
    scope: &ScopeData,
    entry: &Entry,
) -> Option<Node<'n>> {
    if !argument_entry(scope, entry) || entry.writes.is_empty() {
        return None;
    }
    parameter(model, entry.intro_byte)
}
fn argument_entry(scope: &ScopeData, entry: &Entry) -> bool {
    entry.intro_kind == IntroKind::Param
        || scope.kind == ScopeKind::Block && entry.intro_kind == IntroKind::Binding
}
fn candidate<'n>(
    model: &'n FileModel<'_>,
    scope: &ScopeData,
    name: &str,
    entry: &Entry,
    index: &references::Index,
) -> Option<(Node<'n>, bool, bool)> {
    let mut known = true;
    for write in &entry.writes {
        if write.kind == WriteKind::OpAssign {
            known = false;
            continue;
        }
        let target = at(model.tree.root_node(), write.byte)?;
        if pattern_binding(target) {
            continue;
        }
        let node = assignment(target);
        if uses(index, name, node) {
            continue;
        }
        if conditional(node, scope.entered_at) {
            known = false;
            continue;
        }
        return Some((target, known, write.kind == WriteKind::Plain));
    }
    None
}

fn pattern_binding(n: Node<'_>) -> bool {
    ancestors(n)
        .take_while(|p| {
            !matches!(
                p.kind(),
                "assignment"
                    | "operator_assignment"
                    | "method"
                    | "singleton_method"
                    | "block"
                    | "do_block"
                    | "lambda"
            )
        })
        .any(|p| matches!(p.kind(), "match_pattern" | "in_clause"))
}

fn parameter<'n>(model: &'n FileModel<'_>, byte: usize) -> Option<Node<'n>> {
    let n = at(model.tree.root_node(), byte)?;
    let p = ancestors(n).find(|p| p.kind().ends_with("parameters"))?;
    if block_local(p, byte) {
        return None;
    }
    declaration(n, p)
}
fn declaration<'n>(n: Node<'n>, parameters: Node<'n>) -> Option<Node<'n>> {
    let wrappers = ancestors(n)
        .take_while(|a| *a != parameters)
        .collect::<Vec<_>>();
    if wrappers.iter().any(|p| !parameter_wrapper(*p)) {
        return None;
    }
    Some(declared_node(n, &wrappers))
}
fn declared_node<'n>(n: Node<'n>, wrappers: &[Node<'n>]) -> Node<'n> {
    if wrappers
        .iter()
        .any(|p| p.kind() == "destructured_parameter")
    {
        n
    } else {
        wrappers.last().copied().unwrap_or(n)
    }
}

fn parameter_wrapper(n: Node<'_>) -> bool {
    matches!(
        n.kind(),
        "optional_parameter"
            | "keyword_parameter"
            | "block_parameter"
            | "splat_parameter"
            | "hash_splat_parameter"
            | "destructured_parameter"
    )
}

fn block_local(parameters: Node<'_>, byte: usize) -> bool {
    let mut cursor = parameters.walk();
    parameters.children(&mut cursor).enumerate().any(|(i, n)| {
        parameters.field_name_for_child(i as u32) == Some("locals")
            && n.start_byte() <= byte
            && byte < n.end_byte()
    })
}
fn at(root: Node<'_>, byte: usize) -> Option<Node<'_>> {
    root.named_descendant_for_byte_range(byte, byte)
}
fn assignment(n: Node<'_>) -> Node<'_> {
    ancestors(n)
        .find_map(|parent| match parent.kind() {
            "assignment" | "operator_assignment" | "for" => Some(parent),
            "rescue" => Some(n),
            _ => None,
        })
        .unwrap_or(n)
}

fn conditional(n: Node<'_>, stop: usize) -> bool {
    ancestors(n)
        .take_while(|p| p.start_byte() != stop)
        .any(|p| {
            matches!(
                p.kind(),
                "if" | "unless"
                    | "if_modifier"
                    | "unless_modifier"
                    | "conditional"
                    | "while"
                    | "until"
                    | "while_modifier"
                    | "until_modifier"
                    | "case"
                    | "case_match"
                    | "block"
                    | "do_block"
                    | "lambda"
            )
        })
}

fn argument_reads(
    model: &FileModel<'_>,
    scope: usize,
    name: &str,
    entry: &Entry,
    index: &references::Index,
) -> Option<Vec<crate::model::Read>> {
    let reads = references::reads(model, scope, name, entry, index);
    (!reads.is_empty()).then_some(reads)
}
