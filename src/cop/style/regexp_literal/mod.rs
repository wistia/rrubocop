//! Style/RegexpLiteral.

mod rewrite;

use tree_sitter::Node;

use crate::cop::shared::node_bytes;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

use rewrite::{
    PREFERRED_CLOSE, PREFERRED_OPEN, delimiter_pair_ok, regex_body, rewrite_regexp,
    split_body_flags,
};

pub struct RegexpLiteral;

impl Cop for RegexpLiteral {
    fn name(&self) -> &'static str {
        "Style/RegexpLiteral"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["regex"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "slashes");
        let allow_inner = config.get_bool("AllowInnerSlashes", false);
        let b = node_bytes(source, node);
        let is_pct = b.starts_with(b"%r");
        let is_slash = b.starts_with(b"/");
        if !is_pct && !is_slash {
            return;
        }
        let want_percent = want_percent_r(source, node, b, style, allow_inner);
        if is_slash && want_percent && percent_r_delimiters_conflict(b) {
            return;
        }
        report(
            self,
            source,
            node,
            b,
            is_slash,
            is_pct,
            want_percent,
            diagnostics,
            &mut corrections,
        );
    }
}

fn want_percent_r(
    source: &SourceFile,
    node: Node<'_>,
    bytes: &[u8],
    style: &str,
    allow_inner: bool,
) -> bool {
    let multiline = source.offset_to_line_col(node.start_byte()).0
        != source
            .offset_to_line_col(node.end_byte().saturating_sub(1))
            .0;
    let disallowed_slash = !allow_inner && regex_body(bytes).contains(&b'/');
    style == "percent_r"
        || (style == "mixed" && (multiline || disallowed_slash))
        || (style != "percent_r" && style != "mixed" && disallowed_slash)
}

fn percent_r_delimiters_conflict(bytes: &[u8]) -> bool {
    let Some(body) = split_body_flags(bytes).map(|(body, _)| body) else {
        return true;
    };
    !delimiter_pair_ok(&body, PREFERRED_OPEN, PREFERRED_CLOSE)
}

fn report(
    cop: &RegexpLiteral,
    source: &SourceFile,
    node: Node<'_>,
    bytes: &[u8],
    is_slash: bool,
    is_pct: bool,
    want_percent: bool,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let msg = if is_slash && want_percent {
        "Use `%r` around regular expression."
    } else if is_pct && !want_percent {
        "Use `//` around regular expression."
    } else {
        return;
    };
    let (line, col) = source.offset_to_line_col(node.start_byte());
    let mut diag = cop.diagnostic(source, line, col, msg.to_string());
    push_fix(cop, node, bytes, want_percent, corrections, &mut diag);
    diagnostics.push(diag);
}

fn push_fix(
    cop: &RegexpLiteral,
    node: Node<'_>,
    bytes: &[u8],
    to_percent: bool,
    corrections: &mut Option<&mut Vec<Correction>>,
    diag: &mut Diagnostic,
) {
    let Some(corr) = corrections.as_mut() else {
        return;
    };
    let Some(replacement) = rewrite_regexp(bytes, to_percent) else {
        return;
    };
    corr.push(Correction {
        start: node.start_byte(),
        end: node.end_byte(),
        replacement,
        cop_name: cop.name(),
        cop_index: 0,
        owner: None,
    });
    diag.corrected = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RegexpLiteral, "cops/style/regexp_literal");
}
