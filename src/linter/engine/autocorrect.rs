//! RuboCop-style autocorrect loop: re-lint until no corrections apply.

use std::collections::HashSet;
use std::path::Path;

use anyhow::{Result, bail};
use sha2::{Digest, Sha256};

use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::registry::CopRegistry;
use crate::correction::CorrectionSet;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

use super::{LintOutput, lint_source_prepared};
use crate::linter::prepared::{PreparedConfigs, PreparedLint};

pub(crate) const MAX_ITERATIONS: usize = 200;

struct LintCtx<'a> {
    path: &'a Path,
    prepared: PreparedLint<'a>,
    only: Option<&'a [String]>,
    except: &'a [String],
    mode: AutocorrectMode,
    ignore_disable: bool,
    operation: &'a std::sync::Arc<crate::parse::source::LintOperation>,
}

impl<'a> LintCtx<'a> {
    fn lint_bytes(&self, bytes: &[u8]) -> Result<LintOutput> {
        lint_source_prepared(
            &SourceFile::from_bytes(self.path, bytes.to_vec())
                .with_operation(self.operation.clone()),
            &self.prepared,
            self.only,
            self.except,
            self.mode,
            self.ignore_disable,
        )
    }
}

pub(crate) fn lint_file_autocorrect(
    source: &SourceFile,
    prepared: &PreparedLint<'_>,
    only: Option<&[String]>,
    except: &[String],
    mode: AutocorrectMode,
    ignore_disable: bool,
) -> Result<Vec<Diagnostic>> {
    let mut bytes = std::fs::read(&source.path)?;
    let ctx = LintCtx {
        path: &source.path,
        prepared: *prepared,
        only,
        except,
        mode,
        ignore_disable,
        operation: source.operation(),
    };
    let diags = ctx.run(&mut bytes)?;
    if mode != AutocorrectMode::Off {
        std::fs::write(&source.path, &bytes)?;
    }
    Ok(diags)
}

pub(crate) fn lint_bytes_autocorrect(
    path: &Path,
    bytes: &mut Vec<u8>,
    config: &ResolvedConfig,
    registry: &CopRegistry,
    filters: &CopFilterSet,
    only: Option<&[String]>,
    except: &[String],
    mode: AutocorrectMode,
    ignore_disable: bool,
) -> Result<Vec<Diagnostic>> {
    let operation = std::sync::Arc::default();
    let configs = PreparedConfigs::new(config, registry);
    let ctx = LintCtx {
        path,
        prepared: PreparedLint {
            config,
            registry,
            filters,
            configs: &configs,
        },
        only,
        except,
        mode,
        ignore_disable,
        operation: &operation,
    };
    ctx.run(bytes)
}

impl LintCtx<'_> {
    fn run(&self, bytes: &mut Vec<u8>) -> Result<Vec<Diagnostic>> {
        if self.mode == AutocorrectMode::Off {
            return Ok(self.lint_bytes(bytes)?.diagnostics);
        }
        let mut candidate = bytes.clone();
        let diagnostics = run_autocorrect_loop(self, &mut candidate)?;
        *bytes = candidate;
        Ok(diagnostics)
    }
}

fn run_autocorrect_loop(ctx: &LintCtx<'_>, bytes: &mut Vec<u8>) -> Result<Vec<Diagnostic>> {
    let mut corrected = collect_corrected_passes(ctx, bytes, &mut HashSet::new())?;
    corrected.append(&mut final_pass(ctx, bytes)?);
    corrected.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    Ok(corrected)
}

fn collect_corrected_passes(
    ctx: &LintCtx<'_>,
    bytes: &mut Vec<u8>,
    checksums: &mut HashSet<[u8; 32]>,
) -> Result<Vec<Diagnostic>> {
    let mut corrected = Vec::new();
    for _ in 0..MAX_ITERATIONS {
        if !checksums.insert(sha256(bytes)) {
            bail!("Infinite loop detected in {}", ctx.path.display());
        }
        match next_pass(ctx, bytes)? {
            // Remaining offenses come only from `final_pass` — do not return them
            // here or they are duplicated in the `-A` report.
            PassOutcome::Finished => return Ok(corrected),
            PassOutcome::Applied { mut diags, set } => {
                corrected.extend(diags.drain(..).filter(|d| d.corrected));
                *bytes = validated_edits(ctx, bytes, &set)?;
            }
        }
    }
    Ok(corrected)
}

fn validated_edits(ctx: &LintCtx<'_>, bytes: &[u8], set: &CorrectionSet) -> Result<Vec<u8>> {
    let candidate = set.apply(bytes);
    syntax::validate(ctx, &candidate)?;
    Ok(candidate)
}

enum PassOutcome {
    Finished,
    Applied {
        diags: Vec<Diagnostic>,
        set: CorrectionSet,
    },
}

fn next_pass(ctx: &LintCtx<'_>, bytes: &[u8]) -> Result<PassOutcome> {
    let LintOutput {
        diagnostics,
        corrections,
    } = ctx.lint_bytes(bytes)?;
    let Some(set) = corrections
        .filter(|c| !c.is_empty())
        .map(CorrectionSet::from_vec)
        .filter(|s| !s.is_empty())
    else {
        return Ok(PassOutcome::Finished);
    };
    Ok(PassOutcome::Applied {
        diags: diagnostics,
        set,
    })
}

fn final_pass(ctx: &LintCtx<'_>, bytes: &[u8]) -> Result<Vec<Diagnostic>> {
    let off = LintCtx {
        mode: AutocorrectMode::Off,
        ..*ctx
    };
    Ok(off.lint_bytes(bytes)?.diagnostics)
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().into()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::config::ResolvedConfig;
    use crate::cop::registry::CopRegistry;

    pub(super) fn lint_only(path: &str, bytes: &mut Vec<u8>, only: &str) -> Vec<Diagnostic> {
        let config = ResolvedConfig::empty();
        let registry = CopRegistry::default_registry();
        let filters = CopFilterSet::build(&config, &registry);
        let only = [only.to_string()];
        lint_bytes_autocorrect(
            Path::new(path),
            bytes,
            &config,
            &registry,
            &filters,
            Some(&only),
            &[],
            AutocorrectMode::All,
            false,
        )
        .expect("lint")
    }

    #[test]
    fn first_hash_shifts_value_lines() {
        let mut bytes = b"a = {\n    a: [\n      1\n    ]\n}\n".to_vec();
        lint_only("t.rb", &mut bytes, "Layout/FirstHashElementIndentation");
        assert_eq!(bytes, b"a = {\n  a: [\n    1\n  ]\n}\n");
    }

    #[test]
    fn loop_applies_multiple_passes() {
        let mut bytes = b"x = 1  \n  y = 2  \n".to_vec();
        let diags = lint_only("sample.rb", &mut bytes, "Layout/TrailingWhitespace");
        assert_eq!(bytes, b"x = 1\n  y = 2\n");
        assert!(diags.iter().all(|d| d.corrected));
    }

    #[test]
    fn no_duplicate_remaining_offenses() {
        let diags = lint_only(
            "Gemfile",
            &mut b"gem 'b'\ngem 'a'\n".to_vec(),
            "Bundler/OrderedGems",
        );
        let n = diags
            .iter()
            .filter(|d| d.cop_name == "Bundler/OrderedGems")
            .count();
        assert_eq!(n, 1, "remaining offenses must not be duplicated: {diags:?}");
    }

    #[test]
    fn string_literals_autocorrect() {
        let mut bytes = b"x = \"hello\"\n".to_vec();
        let diags = lint_only("sample.rb", &mut bytes, "Style/StringLiterals");
        assert_eq!(bytes, b"x = 'hello'\n");
        assert!(diags.iter().all(|d| d.corrected));
    }

    #[test]
    fn regexp_literal_autocorrect() {
        let mut bytes = b"x = /\\/foo$/i\n".to_vec();
        let diags = lint_only("sample.rb", &mut bytes, "Style/RegexpLiteral");
        assert_eq!(bytes, b"x = %r{/foo$}i\n");
        assert!(diags.iter().all(|d| d.corrected));
    }

    #[test]
    fn reconcile_clears_false_corrected_flags() {
        use crate::correction::{Correction, CorrectionSet, reconcile_corrected};
        use crate::diagnostic::{Diagnostic, Location, Severity};
        use crate::parse::source::SourceFile;

        let source = SourceFile::from_bytes(Path::new("a.rb"), b"foo  \n".to_vec());
        let mut diags = vec![Diagnostic {
            path: "a.rb".into(),
            cop_name: "Layout/TrailingWhitespace".into(),
            message: "m".into(),
            location: Location { line: 1, column: 4 },
            severity: Severity::Convention,
            corrected: true,
            correctable: true,
            source_line: String::new(),
            highlight_length: 1,
        }];
        reconcile_corrected(
            &mut diags,
            &source,
            &CorrectionSet::from_vec(vec![Correction {
                start: 3,
                end: 5,
                replacement: String::new(),
                cop_name: "Layout/Other",
                cop_index: 0,
                owner: None,
            }]),
        );
        assert!(!diags[0].corrected);
    }
}

#[cfg(test)]
mod config_tests;

#[cfg(test)]
mod ownership_tests;

mod syntax;

#[cfg(test)]
mod syntax_tests;

#[cfg(test)]
mod directive_tests;

#[cfg(test)]
mod review_tests;
