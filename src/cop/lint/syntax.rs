//! Lint/Syntax — tree-sitter ERROR/missing plus TargetRubyVersion-gated / MRI-invalid forms.

mod false_errors;
mod numbered_assignment;

use tree_sitter::Node;

use crate::cop::shared::node_bytes;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

use self::false_errors::{endless_eq_offset, mri_valid_false_error};

/// Lint/Syntax — report parse / version syntax issues as fatals (RuboCop parity).
pub struct Syntax;

impl Cop for Syntax {
    fn name(&self) -> &'static str {
        "Lint/Syntax"
    }

    fn default_severity(&self) -> Severity {
        Severity::Fatal
    }

    fn uses_source_phase(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        tree: &tree_sitter::Tree,
        _code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let ruby_ver = config.get_f64("TargetRubyVersion", 2.7);
        let mut nested_endless = false;
        walk(
            source,
            tree.root_node(),
            self,
            ruby_ver,
            0,
            &mut nested_endless,
            diagnostics,
        );
        if nested_endless {
            // Classic parser recovery after nested endless methods often leaves `$end`.
            let (line, _) = source.offset_to_line_col(source.as_bytes().len());
            diagnostics.push(syntax_diag(
                self,
                source,
                line,
                0,
                "unexpected token $end",
                ruby_ver,
            ));
        }
    }
}

fn syntax_diag(
    cop: &Syntax,
    source: &SourceFile,
    line: usize,
    column: usize,
    token_msg: &str,
    ruby_ver: f64,
) -> Diagnostic {
    let ver = format_ruby_ver(ruby_ver);
    cop.diagnostic(
        source,
        line,
        column,
        format!(
            "{token_msg}\n(Using Ruby {ver} parser; configure using `TargetRubyVersion` parameter, under `AllCops`)"
        ),
    )
}

fn format_ruby_ver(v: f64) -> String {
    format!("{v:.1}")
}

fn method_depth_after(node: Node<'_>, depth: usize) -> usize {
    if matches!(node.kind(), "method" | "singleton_method") {
        depth + 1
    } else {
        depth
    }
}

fn check_error(
    source: &SourceFile,
    node: Node<'_>,
    cop: &Syntax,
    ruby_ver: f64,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !(node.is_error() || node.is_missing()) {
        return;
    }
    // Tree-sitter-ruby false parses that MRI accepts (Ruby 2.x/3.x).
    if mri_valid_false_error(source, node) {
        return;
    }
    let (line, col) = source.offset_to_line_col(node.start_byte());
    diagnostics.push(syntax_diag(
        cop,
        source,
        line,
        col,
        "unexpected token",
        ruby_ver,
    ));
}

fn check_endless(
    source: &SourceFile,
    node: Node<'_>,
    cop: &Syntax,
    ruby_ver: f64,
    method_depth: usize,
    nested_endless: &mut bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if ruby_ver >= 3.0 {
        return;
    }
    let Some(eq_off) = endless_eq_offset(source, node) else {
        return;
    };
    let (line, col) = source.offset_to_line_col(eq_off);
    diagnostics.push(syntax_diag(
        cop,
        source,
        line,
        col,
        "unexpected token tEQL",
        ruby_ver,
    ));
    // Nested inside an outer method (depth before entering this node >= 1).
    if method_depth >= 1 {
        *nested_endless = true;
    }
}

fn check_bare_not(
    source: &SourceFile,
    node: Node<'_>,
    cop: &Syntax,
    ruby_ver: f64,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some((line, col)) = bare_not_offense(source, node) else {
        return;
    };
    diagnostics.push(syntax_diag(
        cop,
        source,
        line,
        col,
        "unexpected token tIDENTIFIER",
        ruby_ver,
    ));
}

fn walk(
    source: &SourceFile,
    node: Node<'_>,
    cop: &Syntax,
    ruby_ver: f64,
    method_depth: usize,
    nested_endless: &mut bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let next_depth = method_depth_after(node, method_depth);
    check_error(source, node, cop, ruby_ver, diagnostics);
    check_endless(
        source,
        node,
        cop,
        ruby_ver,
        method_depth,
        nested_endless,
        diagnostics,
    );
    check_bare_not(source, node, cop, ruby_ver, diagnostics);
    if ruby_ver >= 2.7 && numbered_assignment::reserved_assignment(source, node) {
        let (line, col) = source.offset_to_line_col(node.start_byte());
        diagnostics.push(syntax_diag(
            cop,
            source,
            line,
            col,
            "reserved numbered parameter cannot be assigned",
            ruby_ver,
        ));
    }
    let mut cur = node.walk();
    for child in node.children(&mut cur) {
        walk(
            source,
            child,
            cop,
            ruby_ver,
            next_depth,
            nested_endless,
            diagnostics,
        );
    }
}

/// MRI rejects bare `not expr` (no parentheses) outside statement/condition contexts.
fn bare_not_offense(source: &SourceFile, node: Node<'_>) -> Option<(usize, usize)> {
    if node.kind() != "unary" {
        return None;
    }
    let mut cur = node.walk();
    let kids: Vec<Node<'_>> = node.children(&mut cur).collect();
    if kids.len() < 2 {
        return None;
    }
    if node_bytes(source, kids[0]) != b"not" {
        return None;
    }
    let operand = kids[1];
    if operand.kind() == "parenthesized_statements" {
        return None;
    }
    let parent = node.parent()?;
    if bare_not_allowed_parent(source, parent) {
        return None;
    }
    Some(source.offset_to_line_col(operand.start_byte()))
}

fn bare_not_allowed_parent(source: &SourceFile, parent: Node<'_>) -> bool {
    match parent.kind() {
        "program"
        | "body_statement"
        | "block_body"
        | "then"
        | "else"
        | "elsif"
        | "if"
        | "unless"
        | "while"
        | "until"
        | "if_modifier"
        | "unless_modifier"
        | "while_modifier"
        | "until_modifier"
        | "parenthesized_statements"
        | "begin"
        | "do"
        | "rescue"
        | "ensure" => true,
        "binary" => {
            let mut cur = parent.walk();
            parent.children(&mut cur).any(|c| {
                let b = node_bytes(source, c);
                b == b"and" || b == b"or"
            })
        }
        _ => false,
    }
}

/// True when this file has Lint/Syntax fatals that should suppress other cops.
pub fn has_syntax_fatals(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|d| d.cop_name == "Lint/Syntax" && d.severity >= Severity::Error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, run_cop_full, run_cop_full_with_config};
    use std::collections::HashMap;

    #[test]
    fn reserved_numbered_assignments_are_syntax_errors() {
        for code in
            include_str!("../../../tests/fixtures/cops/lint/syntax/offense_numbered_assignments.rb")
                .lines()
        {
            let diags = run_cop_full_with_config(&Syntax, code.as_bytes(), ruby34());
            assert!(has_syntax_fatals(&diags), "{code}: {diags:?}");
        }
    }

    #[test]
    fn numbered_reads_and_nonreserved_assignments_are_valid() {
        for code in include_str!(
            "../../../tests/fixtures/cops/lint/syntax/no_offense_numbered_assignments.rb"
        )
        .lines()
        {
            let diags = run_cop_full_with_config(&Syntax, code.as_bytes(), ruby34());
            assert!(diags.is_empty(), "{code}: {diags:?}");
        }
    }

    fn ruby34() -> CopConfig {
        CopConfig {
            options: HashMap::from([(
                "TargetRubyVersion".into(),
                serde_yml::Value::Number(serde_yml::Number::from(3.4)),
            )]),
            ..CopConfig::default()
        }
    }

    #[test]
    fn no_offense_unicode_symbol() {
        assert_cop_no_offenses_full(
            &Syntax,
            include_bytes!("../../../tests/fixtures/cops/lint/syntax/no_offense_unicode_symbol.rb"),
        );
    }

    #[test]
    fn no_offense_endless_raise() {
        let diags = run_cop_full_with_config(
            &Syntax,
            include_bytes!("../../../tests/fixtures/cops/lint/syntax/no_offense_endless_raise.rb"),
            ruby34(),
        );
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn no_offense_anonymous_block() {
        assert_cop_no_offenses_full(
            &Syntax,
            include_bytes!(
                "../../../tests/fixtures/cops/lint/syntax/no_offense_anonymous_block.rb"
            ),
        );
    }

    #[test]
    fn still_reports_ampersand_not_anonymous_block_forward() {
        let diags = run_cop_full(&Syntax, b"foo(& 1)\n");
        assert!(
            diags.iter().any(|d| d.cop_name == "Lint/Syntax"),
            "`& 1` must still report: {diags:?}"
        );
    }

    #[test]
    fn no_offense_pattern_match() {
        assert_cop_no_offenses_full(
            &Syntax,
            include_bytes!("../../../tests/fixtures/cops/lint/syntax/no_offense_pattern_match.rb"),
        );
    }

    #[test]
    fn still_reports_unclosed_delimiter() {
        let diags = run_cop_full(&Syntax, b"1 + (\n");
        assert!(
            diags.iter().any(|d| d.cop_name == "Lint/Syntax"),
            "unclosed paren must report: {diags:?}"
        );
    }

    #[test]
    fn still_reports_error_after_hash_rocket_not_match_pattern() {
        let diags = run_cop_full(&Syntax, b"h = {\n  a =>\n  !!!\n}\n");
        assert!(
            diags.iter().any(|d| d.cop_name == "Lint/Syntax"),
            "invalid hash after => must report: {diags:?}"
        );
    }

    #[test]
    fn does_not_suppress_error_merely_because_line_has_def_and_eq() {
        let diags = run_cop_full_with_config(&Syntax, b"# def x = y\nfoo(!!!\n", ruby34());
        assert!(
            diags.iter().any(|d| d.cop_name == "Lint/Syntax"),
            "ERROR after comment with def/= must still report: {diags:?}"
        );
    }
}
