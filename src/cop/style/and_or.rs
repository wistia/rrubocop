//! Style/AndOr — ban `and`/`or` (default: only inside conditionals).

use tree_sitter::Node;

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct AndOr;

const CONDITIONAL_KINDS: &[&str] = &[
    "if",
    "unless",
    "while",
    "until",
    "if_modifier",
    "unless_modifier",
    "while_modifier",
    "until_modifier",
];

impl Cop for AndOr {
    fn name(&self) -> &'static str {
        "Style/AndOr"
    }

    fn redundant_disable_audit(&self) -> bool {
        false
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &[
            "binary",
            "if",
            "unless",
            "while",
            "until",
            "if_modifier",
            "unless_modifier",
            "while_modifier",
            "until_modifier",
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let always = config.get_str("EnforcedStyle", "conditionals") == "always";
        if always {
            if node.kind() == "binary" {
                report_ops(self, source, node, diagnostics, &mut corrections);
            }
            return;
        }
        if !CONDITIONAL_KINDS.contains(&node.kind()) {
            return;
        }
        let Some(cond) = node.child_by_field_name("condition") else {
            return;
        };
        walk_condition(self, source, cond, diagnostics, &mut corrections);
    }
}

fn walk_condition(
    cop: &AndOr,
    source: &SourceFile,
    node: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    if node.kind() == "binary" {
        report_ops(cop, source, node, diagnostics, corrections);
    }
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();
    for child in children {
        walk_condition(cop, source, child, diagnostics, corrections);
    }
}

fn report_ops(
    cop: &AndOr,
    source: &SourceFile,
    node: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        push_op(cop, source, child, diagnostics, corrections);
    }
}

fn op_repl(text: &[u8]) -> Option<&'static str> {
    match text {
        b"and" => Some("&&"),
        b"or" => Some("||"),
        _ => None,
    }
}

fn push_correction(
    cop: &AndOr,
    child: Node<'_>,
    repl: &str,
    corrections: &mut Option<&mut Vec<Correction>>,
) -> bool {
    let Some(corr) = corrections.as_deref_mut() else {
        return false;
    };
    corr.push(Correction {
        start: child.start_byte(),
        end: child.end_byte(),
        replacement: repl.to_string(),
        cop_name: cop.name(),
        cop_index: 0,
        owner: None,
    });
    true
}

fn push_op(
    cop: &AndOr,
    source: &SourceFile,
    child: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let text = &source.as_bytes()[child.start_byte()..child.end_byte()];
    let Some(repl) = op_repl(text) else {
        return;
    };
    let (line, col) = source.offset_to_line_col(child.start_byte());
    let word = std::str::from_utf8(text).unwrap_or("");
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        format!("Use `{repl}` instead of `{word}`."),
    );
    if push_correction(cop, child, repl, corrections) {
        diag.corrected = true;
    }
    diagnostics.push(diag);
}
