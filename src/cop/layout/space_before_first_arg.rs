//! Layout/SpaceBeforeFirstArg.

use tree_sitter::Node;

use crate::cop::layout::report;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SpaceBeforeFirstArg;

fn method_node(node: Node<'_>) -> Option<Node<'_>> {
    node.child_by_field_name("method")
        .or_else(|| node.child_by_field_name("name"))
}

fn first_arg(args: Node<'_>) -> Option<Node<'_>> {
    let mut cur = args.walk();
    args.named_children(&mut cur).next()
}

fn gap_spaces(bytes: &[u8], method_end: usize, first_start: usize) -> Option<usize> {
    let between = &bytes[method_end..first_start];
    if between.iter().any(|&b| b == b'\n') {
        return None;
    }
    Some(between.iter().filter(|&&b| b == b' ' || b == b'\t').count())
}

fn line_has_nonspace_at(line: &[u8], col: usize) -> bool {
    line.get(col)
        .is_some_and(|b| !matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
}

fn is_full_line_comment(line: &[u8]) -> bool {
    let mut i = 0;
    while i < line.len() && matches!(line[i], b' ' | b'\t') {
        i += 1;
    }
    line.get(i) == Some(&b'#')
}

/// RuboCop `AllowForAlignment` / `aligned_words?`: arg column matches a word
/// start on another (non-comment) line.
fn aligned_with_adjacent(source: &SourceFile, first_start: usize) -> bool {
    let (line, col) = source.offset_to_line_col(first_start);
    if line == 0 || col == 0 {
        return false;
    }
    struct WordColumns(std::collections::HashMap<usize, usize>);
    let columns = source.cached(0, || {
        let mut columns = std::collections::HashMap::new();
        for bytes in source.lines().filter(|line| !is_full_line_comment(line)) {
            for column in 1..bytes.len() {
                if matches!(bytes[column - 1], b' ' | b'\t') && line_has_nonspace_at(bytes, column)
                {
                    *columns.entry(column).or_insert(0) += 1;
                }
            }
        }
        WordColumns(columns)
    });
    let current = source.line_bytes(line).unwrap_or(b"");
    let current_matches = !is_full_line_comment(current)
        && matches!(current.get(col - 1), Some(b' ') | Some(b'\t'))
        && line_has_nonspace_at(current, col);
    columns.0.get(&col).copied().unwrap_or(0) > usize::from(current_matches)
}

fn offense(source: &SourceFile, node: Node<'_>, allow_align: bool) -> Option<(usize, usize)> {
    let (method_end, first_start) = unparen_call_gap(source, node)?;
    let spaces = gap_spaces(source.as_bytes(), method_end, first_start)?;
    bad_space_gap(source, method_end, first_start, spaces, allow_align)
}

fn unparen_call_gap(source: &SourceFile, node: Node<'_>) -> Option<(usize, usize)> {
    let bytes = source.as_bytes();
    let method = method_node(node)?;
    // `discount:` in a hash/pattern is a label, not `discount` + argument `:`.
    if bytes.get(method.end_byte()) == Some(&b':') {
        return None;
    }
    let args = node.child_by_field_name("arguments")?;
    if bytes.get(args.start_byte()) == Some(&b'(') {
        return None;
    }
    let first = first_arg(args)?;
    Some((method.end_byte(), first.start_byte()))
}

fn bad_space_gap(
    source: &SourceFile,
    method_end: usize,
    first_start: usize,
    spaces: usize,
    allow_align: bool,
) -> Option<(usize, usize)> {
    if spaces == 1 {
        return None;
    }
    if spaces > 1 && allow_align && aligned_with_adjacent(source, first_start) {
        return None;
    }
    Some((method_end, first_start))
}

impl Cop for SpaceBeforeFirstArg {
    fn name(&self) -> &'static str {
        "Layout/SpaceBeforeFirstArg"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "command", "command_call"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some((start, end)) = offense(source, node, config.get_bool("AllowForAlignment", true))
        else {
            return;
        };
        report::report_fix(
            self,
            source,
            start,
            "Put one space between the method name and the first argument.".into(),
            diagnostics,
            &mut corrections,
            start,
            end,
            " ".into(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SpaceBeforeFirstArg, "cops/layout/space_before_first_arg");
    #[test]
    fn cached_alignment_excludes_current_line_and_full_comments() {
        let source = SourceFile::from_bytes("align.rb", b"foo  bar\n#    word\n\n".to_vec());
        assert!(!aligned_with_adjacent(&source, 5));
        let source = SourceFile::from_bytes("align.rb", b"foo  bar\n     word\r\n".to_vec());
        assert!(aligned_with_adjacent(&source, 5));
        let source = SourceFile::from_bytes("align.rb", b"foo  bar\n\xff    word\n".to_vec());
        assert!(aligned_with_adjacent(&source, 5));
    }
}
