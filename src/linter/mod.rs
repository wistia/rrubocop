//! Parallel lint runner.

mod engine;
mod parallel;

use std::path::PathBuf;

use anyhow::Result;

use crate::cache::Cache;
use crate::cli::{Args, AutocorrectMode};
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::registry::CopRegistry;
use crate::diagnostic::{Diagnostic, Severity};

pub use engine::lint_source;

pub(crate) use engine::lint_bytes_autocorrect;

pub struct LintResult {
    pub diagnostics: Vec<Diagnostic>,
    pub files: Vec<PathBuf>,
    /// Paths the walker queued (may exceed `files` under fail-fast).
    pub discovered_count: usize,
}

pub(crate) struct RunPrep {
    pub(crate) filters: CopFilterSet,
    pub(crate) only: Option<Vec<String>>,
    pub(crate) except: Vec<String>,
    pub(crate) mode: AutocorrectMode,
    pub(crate) cache: Option<Cache>,
    /// When false (`--cache false`), skip cache reads but still write results.
    pub(crate) cache_read: bool,
    pub(crate) config_fp: Vec<u8>,
    pub(crate) only_key: String,
    pub(crate) except_key: String,
    pub(crate) ignore_disable: bool,
    pub(crate) force_default_config: bool,
    pub(crate) rubocop_version: String,
    pub(crate) force_exclusion: bool,
    /// `0` = unlimited; otherwise stop once counted offenses reach this.
    pub(crate) fail_fast_limit: u32,
    /// When autocorrecting, only non-corrected offenses count toward the limit.
    pub(crate) fail_fast_uncorrected_only: bool,
}

/// Lint `roots` while walking the tree: analysis starts as soon as paths are queued.
pub fn run_linter(
    args: &Args,
    config: &ResolvedConfig,
    registry: &CopRegistry,
    roots: &[PathBuf],
) -> Result<LintResult> {
    run_linter_with(args, config, registry, roots, |_| {}, |_| {})
}

/// Like [`run_linter`], with `on_discovered(file_count)` when the walk finishes
/// (marks may already have been buffered) and `on_file` per finished path.
pub fn run_linter_with(
    args: &Args,
    config: &ResolvedConfig,
    registry: &CopRegistry,
    roots: &[PathBuf],
    on_discovered: impl FnOnce(usize),
    on_file: impl Fn(&[Diagnostic]) + Sync,
) -> Result<LintResult> {
    let batch = parallel::lint_pipeline(
        &prepare_run(args, config, registry),
        roots,
        config,
        registry,
        on_discovered,
        on_file,
    )?;
    Ok(LintResult {
        diagnostics: batch.diagnostics,
        files: batch.inspected,
        discovered_count: batch.discovered_count,
    })
}

fn prepare_run(args: &Args, config: &ResolvedConfig, registry: &CopRegistry) -> RunPrep {
    let filters = CopFilterSet::build(config, registry);
    let only = optional_cop_list(&args.only, registry);
    let except = expand_cop_list(&args.except, registry);
    let mode = args.autocorrect_mode();
    RunPrep {
        cache: open_result_cache(mode),
        cache_read: args.cache_read_enabled(),
        config_fp: config.cache_fingerprint(),
        except_key: except.join(","),
        only_key: only.as_ref().map(|v| v.join(",")).unwrap_or_default(),
        only,
        except,
        mode,
        filters,
        ignore_disable: args.ignore_disable_comments,
        force_default_config: args.force_default_config,
        rubocop_version: args.rubocop_version.clone().unwrap_or_default(),
        force_exclusion: args.force_exclusion,
        fail_fast_limit: args.fail_fast,
        fail_fast_uncorrected_only: mode != AutocorrectMode::Off,
    }
}

fn optional_cop_list(list: &[String], registry: &CopRegistry) -> Option<Vec<String>> {
    (!list.is_empty()).then(|| expand_cop_list(list, registry))
}

fn open_result_cache(mode: AutocorrectMode) -> Option<Cache> {
    if mode != AutocorrectMode::Off {
        return None;
    }
    let cache = Cache::open()?;
    cache.prune();
    Some(cache)
}

fn expand_cop_list(list: &[String], registry: &CopRegistry) -> Vec<String> {
    let mut out = Vec::new();
    for item in list {
        if item.contains('/') {
            out.push(item.clone());
            continue;
        }
        out.extend(
            registry
                .names()
                .into_iter()
                .filter(|name| name.starts_with(&format!("{item}/")))
                .map(str::to_string),
        );
    }
    out
}

pub fn should_fail(diagnostics: &[Diagnostic], fail_level: &str) -> bool {
    let min = Severity::from_str(fail_level).unwrap_or(Severity::Convention);
    diagnostics
        .iter()
        .any(|d| !d.corrected && d.severity >= min)
}

#[cfg(test)]
mod failure_tests;
