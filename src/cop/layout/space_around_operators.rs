//! Layout/SpaceAroundOperators.

use tree_sitter::{Node, Tree};

use crate::cop::shared;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct SpaceAroundOperators;

fn operators(node: Node<'_>) -> Vec<(Node<'_>, Option<Node<'_>>)> {
    let right = node
        .child_by_field_name("right")
        .or_else(|| node.child_by_field_name("value"))
        .or_else(|| node.child_by_field_name("operand"))
        .or_else(|| {
            let mut cursor = node.walk();
            node.named_children(&mut cursor).last()
        });
    match node.kind() {
        "binary" | "operator_assignment" => node
            .child_by_field_name("operator")
            .map(|op| vec![(op, right)])
            .unwrap_or_default(),
        "assignment" | "pair" | "conditional" | "superclass" | "singleton_class"
        | "exception_variable" | "match_pattern" | "unary" | "block_argument" => {
            let mut cursor = node.walk();
            node.children(&mut cursor)
                .filter(|child| match node.kind() {
                    "assignment" => child.kind() == "=",
                    "pair" | "exception_variable" | "match_pattern" => child.kind() == "=>",
                    "unary" | "block_argument" => {
                        matches!(child.kind(), "-" | "+" | "&")
                            && node.parent().is_some_and(|parent| {
                                parent.kind() == "argument_list"
                                    && parent.parent().is_some_and(|call| {
                                        call.kind() == "call"
                                            && call.child_by_field_name("method").is_some_and(
                                                |method| method.end_byte() == node.start_byte(),
                                            )
                                    })
                            })
                    }
                    "conditional" => matches!(child.kind(), "?" | ":"),
                    "superclass" => child.kind() == "<",
                    "singleton_class" => child.kind() == "<<",
                    _ => false,
                })
                .map(|op| (op, right))
                .collect()
        }
        _ => Vec::new(),
    }
}

fn neighbor_indices(index: usize, len: usize, backwards: bool) -> impl Iterator<Item = usize> {
    let range = if backwards { 0..index } else { index + 1..len };
    range.map(move |step| if backwards { index - step - 1 } else { step })
}

fn horizontal_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r')
}

struct Alignment<'a> {
    source: &'a SourceFile,
    lines: Vec<&'a [u8]>,
    assignments: std::collections::HashSet<usize>,
}

impl<'a> Alignment<'a> {
    fn new(source: &'a SourceFile, root: Node<'_>) -> Self {
        let mut assignments = std::collections::HashSet::new();
        shared::for_each_descendant(root, |node| {
            if matches!(node.kind(), "assignment" | "operator_assignment") {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor).filter(|child| {
                    matches!(
                        child.kind(),
                        "=" | "||="
                            | "&&="
                            | "+="
                            | "-="
                            | "*="
                            | "/="
                            | "%="
                            | "&="
                            | "|="
                            | "^="
                            | "<<="
                            | ">>="
                            | "**="
                    )
                }) {
                    assignments.insert(source.offset_to_line_col(child.start_byte()).0 - 1);
                }
            }
        });
        Self {
            source,
            lines: source.lines().collect(),
            assignments,
        }
    }

    fn indent(&self, line: usize) -> Option<usize> {
        self.lines
            .get(line)?
            .iter()
            .position(|byte| !byte.is_ascii_whitespace())
    }

    fn adjacent(&self, index: usize, indent: Option<usize>) -> Vec<&[u8]> {
        let mut candidates = Vec::new();
        for backwards in [true, false] {
            let indices = neighbor_indices(index, self.lines.len(), backwards);
            for line in indices {
                let Some(current_indent) = self.indent(line) else {
                    continue;
                };
                if self.lines[line][current_indent] == b'#'
                    || indent.is_some_and(|wanted| wanted != current_indent)
                {
                    continue;
                }
                candidates.push(self.lines[line]);
                break;
            }
        }
        candidates
    }

    fn assignment_aligned(operator: &[u8], column: usize, line: &[u8]) -> bool {
        let end = column + operator.len();
        let line = String::from_utf8_lossy(line);
        ((operator.ends_with(b"=") || operator == b"<<")
            && line.chars().nth(end.saturating_sub(1)) == Some('='))
            || (operator.ends_with(b"=")
                && end >= 2
                && line.chars().skip(end - 2).take(2).eq(['<', '<']))
    }

    fn aligned(&self, offset: usize, token: &[u8], words: bool) -> bool {
        let (line, column) = self.source.offset_to_line_col(offset);
        let token = String::from_utf8_lossy(token);
        let token_length = token.chars().count();
        for indent in [None, self.indent(line - 1)] {
            for candidate in self.adjacent(line - 1, indent) {
                let line: Vec<_> = String::from_utf8_lossy(candidate).chars().collect();
                if line
                    .get(column..column + token_length)
                    .is_some_and(|characters| characters.iter().copied().eq(token.chars()))
                    || Self::assignment_aligned(token.as_bytes(), column, candidate)
                    || (words
                        && column > 0
                        && line
                            .get(column - 1)
                            .is_some_and(|character| character.is_whitespace())
                        && line
                            .get(column)
                            .is_some_and(|character| !character.is_whitespace()))
                {
                    return true;
                }
            }
        }
        false
    }

    fn assignment_neighbor(&self, op: Node<'_>, backwards: bool) -> Option<bool> {
        let (line, column) = self.source.offset_to_line_col(op.start_byte());
        let index = line - 1;
        let indent = self.indent(index)?;
        let indices = neighbor_indices(index, self.lines.len(), backwards);
        let mut at_level = true;
        for neighbor in indices {
            let current = self.indent(neighbor);
            if current.is_some_and(|value| value < indent) || (at_level && current.is_none()) {
                break;
            }
            if current == Some(indent) && self.assignments.contains(&neighbor) {
                let operator = &self.source.as_bytes()[op.start_byte()..op.end_byte()];
                return Some(Self::assignment_aligned(
                    operator,
                    column,
                    self.lines[neighbor],
                ));
            }
            if let Some(current) = current {
                at_level = current == indent;
            }
        }
        None
    }

    fn excessive_leading(&self, op: Node<'_>, before: usize, allow: bool) -> bool {
        if before < 2
            || !allow
            || !self.source.as_bytes()[op.start_byte() - before..op.start_byte()].starts_with(b"  ")
        {
            return false;
        }
        if op.parent().is_some_and(|parent| {
            parent.kind() == "assignment"
                || (parent.kind() == "operator_assignment" && matches!(op.kind(), "&&=" | "||="))
        }) {
            self.assignment_neighbor(op, true) != Some(true)
                && self.assignment_neighbor(op, false) == Some(false)
        } else {
            !self.aligned(
                op.start_byte(),
                &self.source.as_bytes()[op.start_byte()..op.end_byte()],
                false,
            )
        }
    }
}

fn check_operator(
    cop: &dyn Cop,
    source: &SourceFile,
    op: Node<'_>,
    right: Option<Node<'_>>,
    config: &CopConfig,
    alignment: &Alignment<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let bytes = source.as_bytes();
    if op
        .parent()
        .is_some_and(|node| node.kind() == "match_pattern")
        && config.get_f64("TargetRubyVersion", 3.4) < 3.0
    {
        return;
    }
    if op.parent().is_some_and(|node| {
        node.kind() == "pair"
            && config.get_str("EnforcedHashRocketStyle", "key") == "table"
            && node.parent().is_some_and(|hash| {
                let mut cursor = hash.walk();
                let pairs: Vec<_> = hash
                    .named_children(&mut cursor)
                    .filter(|child| child.kind() == "pair")
                    .collect();
                pairs
                    .windows(2)
                    .all(|pair| pair[0].start_position().row != pair[1].start_position().row)
            })
    }) {
        return;
    }
    let mut start = op.start_byte();
    while start > 0 && horizontal_space(bytes[start - 1]) {
        start -= 1;
    }
    if start == 0 || bytes[start - 1] == b'\n' {
        return;
    }
    let mut end = op.end_byte();
    while bytes.get(end).is_some_and(|byte| horizontal_space(*byte)) {
        end += 1;
    }
    if bytes.get(end) == Some(&b'#') {
        return;
    }
    let newline = bytes.get(end) == Some(&b'\n');
    if newline {
        end += 1;
    }
    let operator = &bytes[op.start_byte()..op.end_byte()];
    let no_space = (operator == b"**"
        && config.get_str("EnforcedStyleForExponentOperator", "no_space") == "no_space")
        || (operator == b"/"
            && right.is_some_and(|node| {
                matches!(node.kind(), "integer" | "float" | "rational")
                    && bytes[node.start_byte()..node.end_byte()].ends_with(b"r")
            })
            && config.get_str("EnforcedStyleForRationalLiterals", "no_space") == "no_space");
    let before = op.start_byte() - start;
    let after = end - op.end_byte();
    let text = String::from_utf8_lossy(operator);
    let message = if no_space {
        if before == 0 && after == 0 {
            return;
        }
        format!("Space around operator `{text}` detected.")
    } else if before == 0 || after == 0 {
        format!("Surrounding space missing for operator `{text}`.")
    } else if alignment.excessive_leading(op, before, config.get_bool("AllowForAlignment", true))
        || (after > 1
            && !newline
            && bytes[op.end_byte()..end].ends_with(b"  ")
            && (!config.get_bool("AllowForAlignment", true)
                || right.is_none_or(|node| {
                    !alignment.aligned(
                        node.start_byte(),
                        &bytes[node.start_byte()..node.end_byte()],
                        true,
                    )
                })))
    {
        format!("Operator `{text}` should be surrounded by a single space.")
    } else {
        return;
    };
    let (line, column) = source.offset_to_line_col(op.start_byte());
    let mut diagnostic = cop.diagnostic(source, line, column, message);
    diagnostic.highlight_length = operator.len();
    if let Some(corrections) = corrections {
        corrections.push(Correction {
            start,
            end,
            replacement: if no_space {
                text.into_owned()
            } else if newline {
                format!(" {text}\n")
            } else if config.get_bool("ForceEqualSignAlignment", false) && after == 0 {
                format!("{} ", String::from_utf8_lossy(&bytes[start..op.end_byte()]))
            } else {
                format!(" {text} ")
            },
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diagnostic.corrected = true;
    }
    diagnostics.push(diagnostic);
}

impl Cop for SpaceAroundOperators {
    fn name(&self) -> &'static str {
        "Layout/SpaceAroundOperators"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn uses_source_phase(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        tree: &Tree,
        _code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let alignment = Alignment::new(source, tree.root_node());
        shared::for_each_descendant(tree.root_node(), |node| {
            if shared::command_unary_plus(source, node).is_some() {
                return;
            }
            if node.kind() == "binary"
                && node.child_by_field_name("left").is_some_and(|left| {
                    matches!(
                        left.kind(),
                        "return" | "break" | "next" | "redo" | "retry" | "yield"
                    )
                })
            {
                return;
            }
            for (op, right) in operators(node) {
                check_operator(
                    self,
                    source,
                    op,
                    right,
                    config,
                    &alignment,
                    diagnostics,
                    &mut corrections,
                );
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_sibling_operator_spacing_options() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(".rubocop.yml");
        std::fs::write(&path, "Layout/HashAlignment:\n  EnforcedHashRocketStyle: table\nLayout/ExtraSpacing:\n  ForceEqualSignAlignment: true\n").unwrap();
        let config = crate::config::load_config(Some(&path), Some(directory.path()), None).unwrap();
        let options = config.cop_config("Layout/SpaceAroundOperators");
        assert_eq!(options.get_str("EnforcedHashRocketStyle", "key"), "table");
        assert!(options.get_bool("ForceEqualSignAlignment", false));
    }

    #[test]
    fn matches_ruby_operator_regressions_without_source_loss() {
        crate::testutil::assert_original_spec_parity(
            &SpaceAroundOperators,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/layout/space_around_operators/parity.json"
            )),
        );
    }
}
