//! Pinned Alignment/AlignmentCorrector helpers shared by the layout cop ports.
mod display_width;
use crate::cop::Cop;
use crate::cop::shared::{named_kids, node_bytes};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::{Node, Tree};
#[derive(Clone, Copy)]
pub(super) struct Item {
    pub(super) start: usize,
    pub(super) end: usize,
}
struct Interior {
    node: Item,
    inside: Item,
}
pub(super) fn line(source: &SourceFile, start: usize) -> usize {
    source.offset_to_line_col(start).0
}
fn begins_line(source: &SourceFile, start: usize) -> bool {
    let line_start = source.line_start(line(source, start)).unwrap_or(0);
    source.as_bytes()[line_start..start]
        .iter()
        .all(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\x0b' | b'\x0c'))
}
pub(super) fn display_column(source: &SourceFile, start: usize) -> usize {
    let line_start = source.line_start(line(source, start)).unwrap_or(0);
    display_width::of(&String::from_utf8_lossy(
        &source.as_bytes()[line_start..start],
    ))
}
fn shift(
    source: &SourceFile,
    cop_name: &'static str,
    item: Item,
    delta: isize,
    taboo: &[Interior],
    comments: &[(usize, usize)],
    corrections: &mut Vec<Correction>,
) -> bool {
    if comments
        .iter()
        .any(|(start, end)| *start >= item.start && *end <= item.end)
    {
        return false;
    }
    let initial = corrections.len();
    let mut start = item.start;
    while start < item.end {
        let end = if delta > 0 {
            start
        } else if source.as_bytes()[start] == b' ' {
            start.saturating_add((-delta) as usize)
        } else {
            start
        };
        let left = if delta < 0 && source.as_bytes()[start] != b' ' {
            start.checked_sub((-delta) as usize)
        } else {
            Some(start)
        };
        if let Some(left) = left {
            if end <= source.as_bytes().len()
                && !taboo.iter().any(|literal| {
                    literal.node.start >= item.start
                        && literal.node.end <= item.end
                        && left >= literal.inside.start
                        && end <= literal.inside.end
                })
            {
                let allowed = if delta > 0 {
                    !matches!(source.as_bytes().get(start), None | Some(b'\n' | b'\r'))
                } else {
                    left < end
                        && source.as_bytes()[left..end]
                            .iter()
                            .all(|b| matches!(b, b' ' | b'\t'))
                };
                if allowed {
                    corrections.push(Correction {
                        start: left,
                        end,
                        replacement: if delta > 0 {
                            " ".repeat(delta as usize)
                        } else {
                            String::new()
                        },
                        cop_name,
                        cop_index: 0,
                        owner: Some(item.start),
                    });
                }
            }
        }
        let Some(newline) = source.as_bytes()[start..item.end]
            .iter()
            .position(|b| *b == b'\n')
        else {
            break;
        };
        start += newline + 1;
    }
    corrections.len() > initial
}

pub(super) fn nodes(tree: &Tree) -> Vec<Node<'_>> {
    let mut result = Vec::new();
    let mut pending = vec![tree.root_node()];
    while let Some(node) = pending.pop() {
        result.push(node);
        pending.extend(named_kids(node).into_iter().rev());
    }
    result
}
pub(super) struct Investigation<'a, 'b> {
    cop: &'a dyn Cop,
    source: &'a SourceFile,
    diagnostics: &'b mut Vec<Diagnostic>,
    corrections: Option<&'b mut Vec<Correction>>,
    taboo: Vec<Interior>,
    comments: Vec<(usize, usize)>,
    offenses: Vec<Item>,
    initial: usize,
}
impl<'a, 'b> Investigation<'a, 'b> {
    pub(super) fn new(
        cop: &'a dyn Cop,
        source: &'a SourceFile,
        nodes: &[Node<'_>],
        diagnostics: &'b mut Vec<Diagnostic>,
        corrections: Option<&'b mut Vec<Correction>>,
    ) -> Self {
        let mut taboo = Vec::new();
        let mut comments = Vec::new();
        for node in nodes.iter().copied() {
            if matches!(
                node.kind(),
                "string" | "subshell" | "regex" | "delimited_symbol"
            ) && node.child_count() >= 2
            {
                taboo.push(Interior {
                    node: Item {
                        start: node.start_byte(),
                        end: node.end_byte(),
                    },
                    inside: Item {
                        start: node.child(0).unwrap().end_byte(),
                        end: node
                            .child((node.child_count() - 1) as u32)
                            .unwrap()
                            .start_byte(),
                    },
                });
            } else if matches!(node.kind(), "bare_string" | "bare_symbol")
                && node_bytes(source, node).contains(&b'\n')
            {
                taboo.push(Interior {
                    node: Item {
                        start: node.start_byte(),
                        end: node.end_byte(),
                    },
                    inside: Item {
                        start: node.start_byte() + 1,
                        end: node.end_byte(),
                    },
                });
            } else if node.kind() == "heredoc_beginning" {
                if let Some((start, end)) = crate::cop::ported::heredoc_span(source, node) {
                    let end = source.as_bytes()[end..]
                        .iter()
                        .position(|b| *b == b'\n')
                        .map_or(source.as_bytes().len(), |offset| end + offset);
                    taboo.push(Interior {
                        node: Item {
                            start: node.start_byte(),
                            end: node.end_byte(),
                        },
                        inside: Item { start, end },
                    });
                }
            } else if node.kind() == "comment" && node_bytes(source, node).starts_with(b"=begin") {
                comments.push((node.start_byte(), node.end_byte()));
            }
        }
        let initial = corrections.as_ref().map_or(0, |v| v.len());
        Self {
            cop,
            source,
            diagnostics,
            corrections,
            taboo,
            comments,
            offenses: Vec::new(),
            initial,
        }
    }
    pub(super) fn check(&mut self, items: &[Item], base: usize, message: &str) {
        let source = self.source;
        let mut previous = 0;
        for item in items.iter().copied() {
            let current = line(source, item.start);
            let check = current > previous && begins_line(source, item.start);
            previous = current;
            let delta = base as isize - display_column(source, item.start) as isize;
            if !check || delta == 0 {
                continue;
            }
            let (line, column) = source.offset_to_line_col(item.start);
            let mut diagnostic = self.cop.diagnostic(source, line, column, message.into());
            diagnostic.highlight_length =
                String::from_utf8_lossy(&source.as_bytes()[item.start..item.end])
                    .replace("\r\n", "\n")
                    .chars()
                    .count();
            if !self
                .offenses
                .iter()
                .any(|outer| item.start >= outer.start && item.end <= outer.end)
            {
                if let Some(corrections) = self.corrections.as_deref_mut() {
                    diagnostic.corrected = shift(
                        source,
                        self.cop.name(),
                        item,
                        delta,
                        &self.taboo,
                        &self.comments,
                        corrections,
                    );
                }
            }
            self.offenses.push(item);
            self.diagnostics.push(diagnostic);
        }
    }
    pub(super) fn finish(self) {
        if let Some(corrections) = self.corrections {
            crate::cop::shared::normalize_corrected_crlf(
                self.source,
                self.cop.name(),
                corrections,
                self.initial,
            );
        }
    }
}
