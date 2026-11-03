pub mod abc;
pub mod baseline;
pub mod cache;
pub mod cli;
pub mod config;
pub mod cop;
pub mod correction;
pub mod diagnostic;
pub mod formatter;
pub mod fs;
pub mod linter;
pub mod mcp;
pub mod model;
pub mod parse;
pub mod testutil;

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;
use cli::Args;
use cli::AutocorrectMode;
use config::{
    CopFilterSet, ResolvedConfig, load_config, load_default_config, set_rubocop_version_override,
    validate_rubocop_version,
};
use cop::registry::CopRegistry;
use diagnostic::Diagnostic;
use formatter::color::Color;
use formatter::{Formatter, ProgressSink, create_formatter};
use linter::{lint_bytes_autocorrect, run_linter, run_linter_with, should_fail};

pub fn run() -> Result<ExitCode> {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let args = Args::parse_cli();
    if args.mcp {
        return mcp::run();
    }
    set_rubocop_version_override(args.rubocop_version.clone());
    if let Some(version) = &args.rubocop_version {
        validate_rubocop_version(version)?;
    }
    let registry = CopRegistry::default_registry();
    if let Some(code) = list_and_exit(&args, &registry) {
        return Ok(code);
    }

    let config = resolved_config(&args)?;
    if args.stdin.is_some() {
        return lint_stdin(&args, &config, &registry);
    }
    lint_paths(&args, &config, &registry)
}

fn resolved_config(args: &Args) -> Result<ResolvedConfig> {
    let target = configuration_target(&args.paths);
    let mut config = if args.force_default_config {
        let mut cfg = load_default_config(target.as_deref(), None);
        // `--only` may name plugin cops; register departments so filters allow them.
        if !args.only.is_empty() {
            cfg.register_departments_from_only(&args.only);
        }
        cfg
    } else {
        load_config(args.config.as_deref(), target.as_deref(), None)?
    };
    config.apply_display_cli(args.display_cop_names_override());
    Ok(config)
}

fn configuration_target(paths: &[PathBuf]) -> Option<PathBuf> {
    let first = paths.first()?;
    if paths.len() == 1 {
        return Some(first.clone());
    }
    let working_dir = std::env::current_dir().ok()?;
    let directory = |path: &Path| {
        let absolute = working_dir.join(path);
        let path = absolute.canonicalize().unwrap_or(absolute);
        if path.is_file() {
            path.parent().unwrap_or(&path).to_path_buf()
        } else {
            path
        }
    };
    let mut common = directory(first);
    for path in &paths[1..] {
        let directory = directory(path);
        while !directory.starts_with(&common) {
            if !common.pop() {
                return None;
            }
        }
    }
    Some(common)
}

fn list_and_exit(args: &Args, registry: &CopRegistry) -> Option<ExitCode> {
    if args.list_cops {
        for name in registry.names() {
            println!("{name}");
        }
        return Some(ExitCode::SUCCESS);
    }
    if args.list_autocorrectable_cops {
        for cop in registry.cops() {
            if cop.supports_autocorrect() {
                println!("{}", cop.name());
            }
        }
        return Some(ExitCode::SUCCESS);
    }
    None
}

fn exit_from_diags(diags: &[Diagnostic], fail_level: &str) -> ExitCode {
    if should_fail(diags, fail_level) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn print_results(args: &Args, diags: &[Diagnostic], files: &[std::path::PathBuf]) -> ExitCode {
    formatter_for(args).print(diags, files);
    exit_from_diags(diags, &args.fail_level)
}

fn formatter_for(args: &Args) -> Box<dyn Formatter> {
    create_formatter(
        &args.format,
        Color::resolve(args.color_force()),
        args.autocorrect_mode() == AutocorrectMode::Safe,
    )
}

fn list_target_files(args: &Args, config: &ResolvedConfig) -> Result<ExitCode> {
    let discovered = fs::discover_files_filtered(
        &args.paths,
        &CopFilterSet::for_discover(config),
        args.force_exclusion,
    )?;
    for f in &discovered.files {
        println!("{}", f.display());
    }
    Ok(ExitCode::SUCCESS)
}

fn lint_paths(args: &Args, config: &ResolvedConfig, registry: &CopRegistry) -> Result<ExitCode> {
    if args.list_target_files {
        return list_target_files(args, config);
    }
    let fmt = formatter_for(args);
    if fmt.streams_marks() {
        return lint_paths_streaming(args, config, registry, fmt.as_ref());
    }
    let result = run_linter(args, config, registry, &args.paths)?;
    fmt.print(&result.diagnostics, &result.files);
    Ok(exit_from_diags(&result.diagnostics, &args.fail_level))
}

fn lint_paths_streaming(
    args: &Args,
    config: &ResolvedConfig,
    registry: &CopRegistry,
    fmt: &dyn Formatter,
) -> Result<ExitCode> {
    let sink = ProgressSink::new(fmt);
    let result = run_linter_with(
        args,
        config,
        registry,
        &args.paths,
        |n| sink.started(n),
        |diags| sink.file_finished(diags),
    )?;
    sink.finished(&result.diagnostics, &result.files);
    Ok(exit_from_diags(&result.diagnostics, &args.fail_level))
}

fn lint_stdin(args: &Args, config: &ResolvedConfig, registry: &CopRegistry) -> Result<ExitCode> {
    let path = args.stdin.as_ref().unwrap();
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf)?;
    let diags = lint_stdin_source(args, config, registry, path, &mut buf)?;
    let files = if diags.is_some() {
        std::slice::from_ref(path)
    } else {
        &[]
    };
    Ok(print_results(args, diags.as_deref().unwrap_or(&[]), files))
}

fn lint_stdin_source(
    args: &Args,
    config: &ResolvedConfig,
    registry: &CopRegistry,
    path: &std::path::Path,
    bytes: &mut Vec<u8>,
) -> Result<Option<Vec<Diagnostic>>> {
    let filters = CopFilterSet::build(config, registry);
    if args.force_exclusion && filters.is_globally_excluded(path) {
        return Ok(None);
    }
    let only = (!args.only.is_empty()).then(|| args.only.clone());
    let mode = args.autocorrect_mode();
    let diags = lint_bytes_autocorrect(
        path,
        bytes,
        config,
        registry,
        &filters,
        only.as_deref(),
        &args.except,
        mode,
        args.ignore_disable_comments,
    )?;
    if mode != AutocorrectMode::Off && path.is_file() {
        std::fs::write(path, bytes)?;
    }
    Ok(Some(diags))
}

#[cfg(test)]
mod stdin_tests;

#[cfg(test)]
mod config_target_tests;
