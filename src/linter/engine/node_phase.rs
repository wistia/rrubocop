//! Batched AST node-phase walk.

use crate::cli::AutocorrectMode;
use crate::cop::registry::CopRegistry;
use crate::cop::walker::BatchedWalker;
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

use super::syntax_gate::ActiveCop;
use super::{allow_corr, corr_bucket, merge_corrections};

pub(super) fn run_node_phase(
    source: &SourceFile,
    tree: &tree_sitter::Tree,
    active: &[ActiveCop<'_>],
    registry: &CopRegistry,
    mode: AutocorrectMode,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<Vec<Correction>>,
) {
    let ast: Vec<_> = active
        .iter()
        .filter(|(c, _, _)| !c.interested_node_kinds().is_empty())
        .collect();
    if ast.is_empty() {
        return;
    }
    walk_ast_cops(source, tree, &ast, registry, mode, diagnostics, corrections);
}

fn ast_walker<'a>(ast: &[&'a ActiveCop<'a>], mode: AutocorrectMode) -> BatchedWalker<'a> {
    BatchedWalker::with_corr_ok(
        ast.iter().map(|(c, _, _)| *c).collect(),
        ast.iter().map(|(_, c, _)| *c).collect(),
        ast.iter()
            .map(|(c, cfg, _)| allow_corr(mode, *c, cfg))
            .collect(),
    )
}

fn walk_ast_cops(
    source: &SourceFile,
    tree: &tree_sitter::Tree,
    ast: &[&ActiveCop<'_>],
    registry: &CopRegistry,
    mode: AutocorrectMode,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<Vec<Correction>>,
) {
    let mut node_corr = corr_bucket(mode);
    ast_walker(ast, mode).walk(source, tree.root_node(), diagnostics, node_corr.as_mut());
    stamp_node_corrections(registry, &mut node_corr);
    merge_corrections(corrections, node_corr);
}

fn stamp_node_corrections(registry: &CopRegistry, node_corr: &mut Option<Vec<Correction>>) {
    let Some(buf) = node_corr.as_mut() else {
        return;
    };
    for c in buf {
        if let Some(idx) = registry.index_of(c.cop_name) {
            c.cop_index = idx;
        }
    }
}
