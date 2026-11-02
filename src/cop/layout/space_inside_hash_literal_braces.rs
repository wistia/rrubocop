//! Layout/SpaceInsideHashLiteralBraces: pinned token edges, empty and compact styles.
use crate::cop::shared::{named_kids, node_bytes, push_replace, ruby_token_nodes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use tree_sitter::{Node, Tree};

pub struct SpaceInsideHashLiteralBraces;

fn whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n' | b'\x0b' | b'\x0c')
}
fn brace_kind(source: &SourceFile, token: Node<'_>) -> u8 {
    let parent = token.parent().map(|n| n.kind());
    match node_bytes(source, token) {
        b"{" if matches!(parent, Some("hash" | "hash_pattern")) => 1,
        b"}" if matches!(parent, Some("hash" | "hash_pattern" | "block")) => 2,
        _ => 0,
    }
}
struct Investigation<'a, 'b> {
    source: &'a SourceFile,
    diagnostics: &'b mut Vec<Diagnostic>,
    corrections: Option<&'b mut Vec<Correction>>,
    replaced: Vec<(usize, usize)>,
}
impl Investigation<'_, '_> {
    fn report(
        &mut self,
        start: usize,
        end: usize,
        message: String,
        seen: &mut Vec<(usize, usize)>,
    ) {
        if seen.contains(&(start, end)) {
            return;
        }
        seen.push((start, end));
        let source = self.source;
        let bytes = source.as_bytes();
        let raw = &bytes[start..end];
        let (line, column) = source.offset_to_line_col(start);
        let mut diagnostic = SpaceInsideHashLiteralBraces.diagnostic(source, line, column, message);
        diagnostic.highlight_length = String::from_utf8_lossy(raw)
            .replace("\r\n", "\n")
            .chars()
            .count();
        let (left, right, text) = if raw.iter().copied().any(whitespace) {
            (start, end, "")
        } else if raw == b"{" {
            (end, end, " ")
        } else {
            (start, start, " ")
        };
        diagnostic.corrected = push_replace(
            &mut self.corrections,
            left,
            right,
            text,
            SpaceInsideHashLiteralBraces.name(),
        );
        self.replaced.push((left, right));
        self.diagnostics.push(diagnostic);
    }
    fn edge(
        &mut self,
        first: Node<'_>,
        second: Node<'_>,
        config: &CopConfig,
        seen: &mut Vec<(usize, usize)>,
    ) {
        let source = self.source;
        if source.offset_to_line_col(first.start_byte()).0
            < source.offset_to_line_col(second.start_byte()).0
            || second.kind() == "comment"
        {
            return;
        }
        let kind1 = brace_kind(source, first);
        let kind2 = brace_kind(source, second);
        let empty = kind1 == 1 && kind2 == 2;
        let want = if kind1 != 0
            && kind1 == kind2
            && config.get_str("EnforcedStyle", "space") == "compact"
        {
            false
        } else if empty {
            config.get_str("EnforcedStyleForEmptyBraces", "no_space") != "no_space"
        } else {
            config.get_str("EnforcedStyle", "space") != "no_space"
        };
        let bytes = source.as_bytes();
        let has_space = bytes.get(first.end_byte()).is_some_and(|b| whitespace(*b));
        if has_space == want {
            return;
        }
        let brace = if kind1 == 1 { first } else { second };
        let (start, end) = if want {
            (brace.start_byte(), brace.end_byte())
        } else if kind1 == 1 {
            let start = brace.end_byte();
            let mut end = start;
            while bytes.get(end).is_some_and(|b| matches!(b, b' ' | b'\t')) {
                end += 1;
            }
            (start, end)
        } else {
            let end = brace.start_byte();
            let mut start = end;
            while start > 0 && matches!(bytes[start - 1], b' ' | b'\t') {
                start -= 1;
            }
            (start, end)
        };
        let inside = if empty {
            "empty hash literal braces"
        } else if kind1 == 1 {
            "{"
        } else {
            "}"
        };
        self.report(
            start,
            end,
            format!(
                "Space inside {inside} {}.",
                if want { "missing" } else { "detected" }
            ),
            seen,
        );
    }
    fn hash(&mut self, node: Node<'_>, tokens: &[Node<'_>], config: &CopConfig) {
        if tokens.len() < 2
            || tokens[0].parent() != Some(node)
            || tokens
                .last()
                .is_some_and(|token| token.parent() != Some(node))
            || node_bytes(self.source, tokens[0]) != b"{"
            || node_bytes(self.source, *tokens.last().unwrap()) != b"}"
        {
            return;
        }
        let mut seen = Vec::new();
        self.edge(tokens[0], tokens[1], config, &mut seen);
        if tokens.len() > 2 {
            self.edge(
                tokens[tokens.len() - 2],
                tokens[tokens.len() - 1],
                config,
                &mut seen,
            );
        }
        if config.get_str("EnforcedStyleForEmptyBraces", "no_space") == "no_space" {
            let start = node.start_byte() + 1;
            let end = node.end_byte() - 1;
            if start < end
                && self.source.as_bytes()[start..end]
                    .iter()
                    .copied()
                    .all(whitespace)
            {
                self.report(
                    start,
                    end,
                    "Space inside empty hash literal braces detected.".into(),
                    &mut seen,
                );
            }
        }
    }
}
impl Cop for SpaceInsideHashLiteralBraces {
    fn name(&self) -> &'static str {
        "Layout/SpaceInsideHashLiteralBraces"
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
        _map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<Correction>>,
    ) {
        let mut hashes = Vec::new();
        let mut pending = vec![tree.root_node()];
        while let Some(node) = pending.pop() {
            if node.kind() == "hash"
                || node.kind() == "hash_pattern"
                    && !config
                        .get_str("__RubocopVersion", "1.91.0")
                        .starts_with("1.65.")
            {
                hashes.push(node);
            }
            pending.extend(named_kids(node).into_iter().rev());
        }
        if hashes.is_empty() {
            return;
        }
        let mut tokens = ruby_token_nodes(source, tree.root_node());
        tokens.sort_by_key(|n| (n.start_byte(), n.end_byte()));
        let initial_count = diagnostics.len();
        let mut investigation = Investigation {
            source,
            diagnostics,
            corrections,
            replaced: Vec::new(),
        };
        for hash in hashes {
            let start = tokens.partition_point(|n| n.start_byte() < hash.start_byte());
            let within: Vec<_> = tokens[start..]
                .iter()
                .copied()
                .take_while(|n| n.start_byte() < hash.end_byte())
                .filter(|n| n.end_byte() <= hash.end_byte())
                .collect();
            investigation.hash(hash, &within, config);
        }
        if investigation.diagnostics.len() > initial_count {
            for (start, window) in source.as_bytes().windows(2).enumerate() {
                if window == b"\r\n"
                    && !investigation
                        .replaced
                        .iter()
                        .any(|(left, right)| start >= *left && start < *right)
                {
                    push_replace(
                        &mut investigation.corrections,
                        start,
                        start + 1,
                        "",
                        self.name(),
                    );
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        SpaceInsideHashLiteralBraces,
        "cops/layout/space_inside_hash_literal_braces"
    );
    #[test]
    fn independent_original_states() {
        crate::testutil::assert_original_spec_parity(
            &SpaceInsideHashLiteralBraces,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/layout/space_inside_hash_literal_braces/independent-parity.json"
            )),
        );
    }
    #[test]
    fn original_ruby_states() {
        crate::testutil::assert_original_spec_parity(
            &SpaceInsideHashLiteralBraces,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/layout/space_inside_hash_literal_braces/original-parity.json"
            )),
        );
    }
}
