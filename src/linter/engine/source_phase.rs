//! Line / source / file-model lint phases.

use crate::cli::AutocorrectMode;
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::model;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

use super::syntax_gate::ActiveCop;
use super::{allow_corr, finish_cop_pass};

pub(super) fn run_line_phase(
    source: &SourceFile,
    active: &[ActiveCop<'_>],
    mode: AutocorrectMode,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<Vec<Correction>>,
) {
    for (cop, cfg, idx) in active.iter().filter(|(c, _, _)| c.uses_line_phase()) {
        let mut corr_buf = allow_corr(mode, *cop, cfg).then(Vec::new);
        let before = diagnostics.len();
        cop.check_lines(source, cfg, diagnostics, corr_buf.as_mut());
        finish_cop_pass(
            source,
            diagnostics,
            before,
            cfg,
            &mut corr_buf,
            *idx,
            corrections,
        );
    }
}

pub(super) fn run_source_phase(
    source: &SourceFile,
    tree: &tree_sitter::Tree,
    active: &[ActiveCop<'_>],
    mode: AutocorrectMode,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<Vec<Correction>>,
) {
    run_file_model_cops(source, tree, active, mode, diagnostics, corrections);
    run_plain_source_cops(source, tree, active, mode, diagnostics, corrections);
}

fn run_file_model_cops(
    source: &SourceFile,
    tree: &tree_sitter::Tree,
    active: &[ActiveCop<'_>],
    mode: AutocorrectMode,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<Vec<Correction>>,
) {
    let model_cops: Vec<_> = active
        .iter()
        .filter(|(c, _, _)| c.needs_file_model())
        .collect();
    if model_cops.is_empty() {
        return;
    }
    let file_model = model::build(source.as_bytes(), tree.clone());
    seed_local_identifiers(source, &file_model, active);
    for (cop, cfg, idx) in model_cops {
        one_file_model_cop(
            source,
            &file_model,
            *cop,
            cfg,
            *idx,
            mode,
            diagnostics,
            corrections,
        );
    }
}

fn one_file_model_cop(
    source: &SourceFile,
    file_model: &crate::model::FileModel,
    cop: &dyn crate::cop::Cop,
    cfg: &crate::cop::CopConfig,
    idx: usize,
    mode: AutocorrectMode,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<Vec<Correction>>,
) {
    let mut corr_buf = allow_corr(mode, cop, cfg).then(Vec::new);
    let before = diagnostics.len();
    cop.check_file_model(source, file_model, cfg, diagnostics, corr_buf.as_mut());
    finish_cop_pass(
        source,
        diagnostics,
        before,
        cfg,
        &mut corr_buf,
        idx,
        corrections,
    );
}

fn run_plain_source_cops(
    source: &SourceFile,
    tree: &tree_sitter::Tree,
    active: &[ActiveCop<'_>],
    mode: AutocorrectMode,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<Vec<Correction>>,
) {
    let plain: Vec<_> = active
        .iter()
        .filter(|(c, _, _)| c.uses_source_phase() && !c.needs_file_model())
        .collect();
    if plain.is_empty() {
        return;
    }
    // Build CodeMap once, only when plain source cops need it.
    let code_map = CodeMap::from_tree(tree.root_node(), source.as_bytes());
    for (cop, cfg, idx) in plain {
        one_plain_source_cop(
            source,
            tree,
            &code_map,
            *cop,
            cfg,
            *idx,
            mode,
            diagnostics,
            corrections,
        );
    }
}

fn one_plain_source_cop(
    source: &SourceFile,
    tree: &tree_sitter::Tree,
    code_map: &CodeMap,
    cop: &dyn crate::cop::Cop,
    cfg: &crate::cop::CopConfig,
    idx: usize,
    mode: AutocorrectMode,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<Vec<Correction>>,
) {
    let mut corr_buf = allow_corr(mode, cop, cfg).then(Vec::new);
    let before = diagnostics.len();
    cop.check_source(source, tree, code_map, cfg, diagnostics, corr_buf.as_mut());
    finish_cop_pass(
        source,
        diagnostics,
        before,
        cfg,
        &mut corr_buf,
        idx,
        corrections,
    );
}

fn uses_local_identifiers(name: &str) -> bool {
    matches!(
        name,
        "Cops/NoSleep"
            | "Cops/NoForeignKeys"
            | "AnyCable/PeriodicalTimers"
            | "Style/TopLevelMethodDefinition"
            | "Lint/SelfAssignment"
            | "Lint/SafeNavigationWithEmpty"
            | "Cops/RescueMustReportError"
            | "Lint/OutOfRangeRegexpRef"
            | "Style/EvalWithLocation"
            | "Cops/NoSetex"
    )
}

fn seed_local_identifiers(
    source: &SourceFile,
    file_model: &model::FileModel<'_>,
    active: &[ActiveCop<'_>],
) {
    if active
        .iter()
        .any(|(cop, _, _)| uses_local_identifiers(cop.name()))
    {
        source.cached(0, || model::LocalIdentifiers::from_model(file_model));
    }
}
