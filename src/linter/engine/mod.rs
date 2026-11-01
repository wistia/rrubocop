//! Per-file cop execution (line / source / AST phases).

mod autocorrect;
mod node_phase;
mod offense;
mod source_phase;
mod syntax_gate;

use anyhow::Result;

use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::registry::CopRegistry;
use crate::cop::{Cop, CopConfig};
use crate::correction::{Correction, CorrectionSet};
use crate::diagnostic::Diagnostic;
use crate::parse;
use crate::parse::directives;
use crate::parse::source::SourceFile;

use super::prepared::{PreparedConfigs, PreparedLint};

type ActiveCop<'a> = syntax_gate::ActiveCop<'a>;

pub struct LintOutput {
    pub diagnostics: Vec<Diagnostic>,
    pub corrections: Option<Vec<Correction>>,
}

pub(crate) use autocorrect::{lint_bytes_autocorrect, lint_file_autocorrect};

pub fn lint_source(
    source: &SourceFile,
    config: &ResolvedConfig,
    registry: &CopRegistry,
    filters: &CopFilterSet,
    only: Option<&[String]>,
    except: &[String],
    mode: AutocorrectMode,
    ignore_disable: bool,
) -> Result<LintOutput> {
    let configs = PreparedConfigs::new(config, registry);
    let context = PreparedLint {
        config,
        registry,
        filters,
        configs: &configs,
    };
    lint_source_prepared(source, &context, only, except, mode, ignore_disable)
}

pub(super) fn lint_source_prepared(
    source: &SourceFile,
    context: &PreparedLint<'_>,
    only: Option<&[String]>,
    except: &[String],
    mode: AutocorrectMode,
    ignore_disable: bool,
) -> Result<LintOutput> {
    let registry = context.registry;
    let active = select_active(source, *context, only, except);
    if active.is_empty() {
        return Ok(LintOutput {
            diagnostics: Vec::new(),
            corrections: None,
        });
    }
    let tree = parse::parse_ruby(source)?;
    let mut diagnostics = Vec::new();
    let mut corrections = corr_bucket(mode);
    run_file_cops(
        source,
        &tree,
        context,
        registry,
        &active,
        mode,
        &mut diagnostics,
        &mut corrections,
    )?;
    apply_directives(
        source,
        context,
        registry,
        &active,
        mode,
        ignore_disable,
        &mut diagnostics,
        &mut corrections,
    );
    finalize_corrections(source, &mut diagnostics, &mut corrections);
    Ok(LintOutput {
        diagnostics,
        corrections,
    })
}

fn finalize_corrections(
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<Vec<Correction>>,
) {
    let Some(corrs) = corrections.as_mut() else {
        return;
    };
    let set = CorrectionSet::from_vec(std::mem::take(corrs));
    crate::correction::reconcile_corrected(diagnostics, source, &set);
    *corrs = set.accepted().to_vec();
}

fn run_file_cops(
    source: &SourceFile,
    tree: &tree_sitter::Tree,
    context: &PreparedLint<'_>,
    registry: &CopRegistry,
    active: &[ActiveCop<'_>],
    mode: AutocorrectMode,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<Vec<Correction>>,
) -> Result<()> {
    if syntax_gate::run(
        source,
        tree,
        context,
        active,
        mode,
        diagnostics,
        corrections,
    ) {
        return Ok(());
    }
    run_non_syntax(
        source,
        tree,
        active,
        registry,
        mode,
        diagnostics,
        corrections,
    );
    Ok(())
}

fn run_redundant_disable_audit(
    source: &SourceFile,
    context: &PreparedLint<'_>,
    pre_filter: &[Diagnostic],
    active: &[ActiveCop<'_>],
    mode: AutocorrectMode,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<Vec<Correction>>,
) {
    let active_refs = audit_configs(active);
    for (cop, cfg, idx) in active {
        if cop.name() != "Lint/RedundantCopDisableDirective" {
            continue;
        }
        let mut corr_buf = allow_corr(mode, *cop, cfg).then(Vec::new);
        let before = diagnostics.len();
        cop.audit_after_cops(
            source,
            pre_filter,
            &active_refs,
            diagnostics,
            corr_buf.as_mut(),
        );
        finish_cop_pass(
            source,
            diagnostics,
            before,
            context.root_config(*idx),
            &mut corr_buf,
            *idx,
            corrections,
        );
        break;
    }
}

fn audit_configs<'a>(active: &[ActiveCop<'a>]) -> Vec<(&'a dyn Cop, &'a CopConfig)> {
    active
        .iter()
        .map(|(cop, config, _)| (*cop, *config))
        .collect()
}

fn apply_directives(
    source: &SourceFile,
    context: &PreparedLint<'_>,
    registry: &CopRegistry,
    active: &[ActiveCop<'_>],
    mode: AutocorrectMode,
    ignore_disable: bool,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<Vec<Correction>>,
) {
    let pre_filter = diagnostics.clone();
    filter_directives(source, ignore_disable, diagnostics);
    if !ignore_disable {
        run_redundant_disable_audit(
            source,
            context,
            &pre_filter,
            active,
            mode,
            diagnostics,
            corrections,
        );
    }
    filter_directives(source, ignore_disable, diagnostics);
    if let Some(edits) = corrections.as_mut() {
        crate::correction::filter_disabled(source, edits, ignore_disable);
    }
    offense::finalize_offenses(source, context.config, registry, diagnostics);
}

fn run_non_syntax(
    source: &SourceFile,
    tree: &tree_sitter::Tree,
    active: &[ActiveCop<'_>],
    registry: &CopRegistry,
    mode: AutocorrectMode,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<Vec<Correction>>,
) {
    run_phases(
        source,
        tree,
        &active
            .iter()
            .filter(|(c, _, _)| c.name() != "Lint/Syntax")
            .map(|(c, cfg, idx)| (*c, *cfg, *idx))
            .collect::<Vec<_>>(),
        registry,
        mode,
        diagnostics,
        corrections,
    );
}

fn run_phases(
    source: &SourceFile,
    tree: &tree_sitter::Tree,
    active: &[ActiveCop<'_>],
    registry: &CopRegistry,
    mode: AutocorrectMode,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<Vec<Correction>>,
) {
    source_phase::run_line_phase(source, active, mode, diagnostics, corrections);
    source_phase::run_source_phase(source, tree, active, mode, diagnostics, corrections);
    node_phase::run_node_phase(
        source,
        tree,
        active,
        registry,
        mode,
        diagnostics,
        corrections,
    );
}

fn filter_directives(source: &SourceFile, ignore_disable: bool, diagnostics: &mut Vec<Diagnostic>) {
    if ignore_disable {
        return;
    }
    let dirs = directives::parse(&String::from_utf8_lossy(source.as_bytes()));
    diagnostics.retain(|d| !dirs.suppresses(&d.cop_name, d.location.line));
}

fn select_active<'a>(
    source: &'a SourceFile,
    context: PreparedLint<'a>,
    only: Option<&[String]>,
    except: &[String],
) -> Vec<ActiveCop<'a>> {
    let path = source.path.as_path();
    let configs = context.for_file(path);
    context
        .registry
        .cops()
        .iter()
        .enumerate()
        .filter(|(index, cop)| {
            cop_wanted(cop.name(), path, context.filters, only, except) && !configs.disabled(*index)
        })
        .map(|(index, cop)| (cop.as_ref(), configs.get(index), index))
        .collect()
}

fn cop_wanted(
    name: &str,
    path: &std::path::Path,
    filters: &CopFilterSet,
    only: Option<&[String]>,
    except: &[String],
) -> bool {
    if only.is_some_and(|o| !o.iter().any(|n| n == name)) {
        return false;
    }
    if except.iter().any(|e| e == name) {
        return false;
    }
    filters.is_cop_enabled_for_file(name, path)
}

pub(super) fn corr_bucket(mode: AutocorrectMode) -> Option<Vec<Correction>> {
    (mode != AutocorrectMode::Off).then(Vec::new)
}

pub(super) fn allow_corr(mode: AutocorrectMode, cop: &dyn Cop, cfg: &CopConfig) -> bool {
    if cfg.autocorrect_disabled() {
        return false;
    }
    match mode {
        AutocorrectMode::Off => false,
        AutocorrectMode::Safe => {
            cop.supports_autocorrect()
                && cfg.get_bool("Safe", true)
                && cfg.get_bool("SafeAutoCorrect", cop.safe_autocorrect())
        }
        AutocorrectMode::All => cop.supports_autocorrect(),
    }
}

pub(super) fn merge_corrections(into: &mut Option<Vec<Correction>>, from: Option<Vec<Correction>>) {
    if let (Some(all), Some(buf)) = (into.as_mut(), from) {
        all.extend(buf);
    }
}

pub(super) fn finish_cop_pass(
    source: &SourceFile,
    diagnostics: &mut [Diagnostic],
    before: usize,
    cfg: &CopConfig,
    corr_buf: &mut Option<Vec<Correction>>,
    idx: usize,
    corrections: &mut Option<Vec<Correction>>,
) {
    if let Some(buf) = corr_buf.as_mut() {
        crate::correction::assign_owners(source, &diagnostics[before..], buf);
        for c in buf {
            c.cop_index = idx;
        }
    }
    merge_corrections(corrections, corr_buf.take());
    if let Some(sev) = cfg.severity {
        for d in &mut diagnostics[before..] {
            d.severity = sev;
        }
    }
}
