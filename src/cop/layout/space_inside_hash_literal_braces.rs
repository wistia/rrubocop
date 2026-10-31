//! Layout/SpaceInsideHashLiteralBraces.

use tree_sitter::Node;

use crate::cop::layout::space_delim;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SpaceInsideHashLiteralBraces;

fn side_msg(want: bool, brace: &str) -> String {
    let problem = if want { "missing" } else { "detected" };
    format!("Space inside {brace} {problem}.")
}

fn check_open(
    cop: &dyn Cop,
    source: &SourceFile,
    bytes: &[u8],
    d: &space_delim::DelimSpace,
    open_off: usize,
    want: bool,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
    correctable: &mut bool,
) {
    let msg = side_msg(want, "{");
    if want {
        space_delim::add_space_after(cop, source, d, open_off, msg, diagnostics, corrections);
    } else {
        space_delim::strip_space_after(
            cop,
            source,
            bytes,
            d,
            msg,
            diagnostics,
            corrections,
            correctable,
        );
    }
}

fn check_close(
    cop: &dyn Cop,
    source: &SourceFile,
    bytes: &[u8],
    d: &space_delim::DelimSpace,
    want: bool,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
    correctable: &mut bool,
) {
    let msg = side_msg(want, "}");
    if want {
        space_delim::add_space_before(cop, source, d, d.inner_e, msg, diagnostics, corrections);
    } else {
        space_delim::strip_space_before(
            cop,
            source,
            bytes,
            d,
            msg,
            diagnostics,
            corrections,
            correctable,
        );
    }
}

fn hash_inner(source: &SourceFile, node: Node<'_>) -> Option<space_delim::DelimSpace> {
    let bytes = source.as_bytes();
    if bytes.get(node.start_byte()) != Some(&b'{') {
        return None;
    }
    let start = node.start_byte() + 1;
    let end = node.end_byte().checked_sub(1)?;
    if end <= start {
        return None;
    }
    space_delim::scan_inner(bytes, start, end)
}

fn opening_comment(source: &SourceFile, offset: usize) -> bool {
    source.as_bytes()[offset..]
        .iter()
        .copied()
        .skip_while(|b| matches!(b, b' ' | b'\t'))
        .next()
        == Some(b'#')
}

fn spacing_context(
    source: &SourceFile,
    node: Node<'_>,
    config: &CopConfig,
) -> Option<(space_delim::DelimSpace, bool)> {
    if node.kind() == "hash_pattern"
        && config
            .get_str("__RubocopVersion", "1.91.0")
            .starts_with("1.65.")
    {
        return None;
    }
    let inner = hash_inner(source, node)?;
    let want = config.get_str("EnforcedStyle", "space") != "no_space";
    if !want && opening_comment(source, inner.inner_s) {
        return None;
    }
    Some((inner, want))
}

impl Cop for SpaceInsideHashLiteralBraces {
    fn name(&self) -> &'static str {
        "Layout/SpaceInsideHashLiteralBraces"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["hash", "hash_pattern"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some((d, want)) = spacing_context(source, node, config) else {
            return;
        };
        let bytes = source.as_bytes();
        let mut correctable = true;
        check_open(
            self,
            source,
            bytes,
            &d,
            node.start_byte(),
            want,
            diagnostics,
            &mut corrections,
            &mut correctable,
        );
        check_close(
            self,
            source,
            bytes,
            &d,
            want,
            diagnostics,
            &mut corrections,
            &mut correctable,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        SpaceInsideHashLiteralBraces,
        "cops/layout/space_inside_hash_literal_braces"
    );
}
