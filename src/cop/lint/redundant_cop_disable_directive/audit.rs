use super::RedundantCopDisableDirective;
use super::directives::{
    DisableDirective, collect_directives, cop_names, disable_marker, offense_span,
};
use super::removal::push_removal;
use crate::cop::{Cop, CopConfig, cop_ran_in_lint};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

fn cop_matches_disabled(offense_cop: &str, disabled: &str) -> bool {
    if disabled.eq_ignore_ascii_case("all") {
        return true;
    }
    if disabled.eq_ignore_ascii_case(offense_cop) {
        return true;
    }
    offense_cop
        .split_once('/')
        .is_some_and(|(dept, _)| disabled.eq_ignore_ascii_case(dept))
}

fn offense_in_range(offense: &Diagnostic, range: (usize, usize)) -> bool {
    offense.location.line >= range.0 && offense.location.line <= range.1
}

fn directive_needed(
    source: &SourceFile,
    cop: &str,
    dir: &DisableDirective,
    offenses: &[Diagnostic],
) -> bool {
    let directives = crate::parse::directives::cached(source);
    offenses.iter().any(|o| {
        cop_matches_disabled(&o.cop_name, cop)
            && offense_in_range(o, dir.range)
            && directives.disabled_by(&o.cop_name, o.location.line, dir.line)
    })
}

fn active_for_redundant_audit(c: &dyn Cop) -> bool {
    cop_ran_in_lint(c) && c.redundant_disable_audit()
}

fn cop_in_department(c: &dyn Cop, dept: &str) -> bool {
    c.name()
        .split_once('/')
        .is_some_and(|(d, _)| d.eq_ignore_ascii_case(dept))
}

fn any_active_cop(active: &[(&dyn Cop, &CopConfig)], pred: impl Fn(&dyn Cop) -> bool) -> bool {
    active.iter().any(|(c, _)| pred(*c))
}

fn cop_auditable(name: &str, active: &[(&dyn Cop, &CopConfig)]) -> bool {
    if name.eq_ignore_ascii_case("all") {
        return any_active_cop(active, active_for_redundant_audit);
    }
    if name.contains('/') {
        return any_active_cop(active, |c| {
            c.name().eq_ignore_ascii_case(name) && active_for_redundant_audit(c)
        });
    }
    any_active_cop(active, |c| {
        active_for_redundant_audit(c) && cop_in_department(c, name)
    })
}

fn cop_is_redundant(
    source: &SourceFile,
    name: &str,
    dir: &DisableDirective,
    offenses: &[Diagnostic],
    active: &[(&dyn Cop, &CopConfig)],
) -> bool {
    cop_auditable(name, active)
        && (already_disabled(source, name, dir.line)
            || !directive_needed(source, name, dir, offenses))
}

fn redundant_offense(
    cop: &RedundantCopDisableDirective,
    source: &SourceFile,
    line: &str,
    dir: &DisableDirective,
    name: &str,
) -> Diagnostic {
    let (col, length) = offense_span(line, dir, name);
    let mut diag = cop.diagnostic(source, dir.line, col, redundant_message(name));
    diag.highlight_length = length;
    diag
}

fn block_disable_cops(source: &SourceFile, line_no: usize) -> Option<Vec<String>> {
    let directives = crate::parse::directives::cached(source);
    let line = directives.comment_line(source, line_no)?;
    if !line.trim_start().starts_with('#') {
        return None;
    }
    let (_, rest) = disable_marker(&line)?;
    Some(cop_names(rest))
}

fn all_block_disable_cops_redundant(
    source: &SourceFile,
    dir: &DisableDirective,
    offenses: &[Diagnostic],
    active: &[(&dyn Cop, &CopConfig)],
) -> bool {
    block_disable_cops(source, dir.line).is_some_and(|cops| {
        cops.iter()
            .all(|name| cop_is_redundant(source, name, dir, offenses, active))
    })
}

fn remove_entire_disable_line(
    source: &SourceFile,
    dir: &DisableDirective,
    offenses: &[Diagnostic],
    active: &[(&dyn Cop, &CopConfig)],
    remove_entire: bool,
) -> bool {
    if !remove_entire {
        return false;
    }
    if dir.range.0 == dir.range.1 {
        return true;
    }
    all_block_disable_cops_redundant(source, dir, offenses, active)
}

fn apply_redundant_fix(
    source: &SourceFile,
    dir: &DisableDirective,
    name: &str,
    remove_entire: bool,
    entire_fix: &mut bool,
    offenses: &[Diagnostic],
    active: &[(&dyn Cop, &CopConfig)],
    mut corrections: Option<&mut Vec<Correction>>,
    diag: &mut Diagnostic,
) {
    if remove_entire_disable_line(source, dir, offenses, active, remove_entire) {
        if !*entire_fix {
            push_removal(
                source,
                dir.line,
                name,
                None,
                corrections.as_deref_mut(),
                diag,
            );
            *entire_fix = true;
        } else if corrections.is_some() {
            diag.corrected = true;
        }
        return;
    }
    push_removal(
        source,
        dir.line,
        name,
        Some(1),
        corrections.as_deref_mut(),
        diag,
    );
}

fn report_directive_redundancies(
    cop: &RedundantCopDisableDirective,
    source: &SourceFile,
    line: &str,
    dir: &DisableDirective,
    offenses: &[Diagnostic],
    active: &[(&dyn Cop, &CopConfig)],
    diagnostics: &mut Vec<Diagnostic>,
    mut corrections: Option<&mut Vec<Correction>>,
) {
    let redundant: Vec<&String> = dir
        .cops
        .iter()
        .filter(|name| cop_is_redundant(source, name, dir, offenses, active))
        .collect();
    if redundant.is_empty() {
        return;
    }
    let remove_entire = redundant.len() == dir.cops.len();
    let mut entire_fix = false;
    for name in redundant {
        let mut diag = redundant_offense(cop, source, line, dir, name);
        apply_redundant_fix(
            source,
            dir,
            name,
            remove_entire,
            &mut entire_fix,
            offenses,
            active,
            corrections.as_deref_mut(),
            &mut diag,
        );
        diagnostics.push(diag);
    }
}

/// Post-pass: compare disable directives against offenses found before filtering.
pub fn audit_redundant_disables(
    cop: &RedundantCopDisableDirective,
    source: &SourceFile,
    offenses: &[Diagnostic],
    active: &[(&dyn Cop, &CopConfig)],
    diagnostics: &mut Vec<Diagnostic>,
    mut corrections: Option<&mut Vec<Correction>>,
) {
    for dir in collect_directives(source) {
        let directives = crate::parse::directives::cached(source);
        let line = directives
            .comment_line(source, dir.line)
            .unwrap_or_default();
        report_directive_redundancies(
            cop,
            source,
            &line,
            &dir,
            offenses,
            active,
            diagnostics,
            corrections.as_deref_mut(),
        );
    }
}

fn redundant_message(name: &str) -> String {
    if name.eq_ignore_ascii_case("all") {
        return "Unnecessary disabling of all cops.".into();
    }
    if !name.contains('/') {
        return format!("Unnecessary disabling of `{name}` department.");
    }
    format!("Unnecessary disabling of `{name}`.")
}

fn already_disabled(source: &SourceFile, name: &str, line: usize) -> bool {
    name.contains('/') && crate::parse::directives::cached(source).restarts_block(name, line)
}
