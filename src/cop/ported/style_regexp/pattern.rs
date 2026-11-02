use super::super::context::*;
use super::{options, scanner};
use std::sync::{Arc, OnceLock};
use tree_sitter::Node;
#[derive(Default)]
struct Analysis {
    tokens: OnceLock<Vec<scanner::Token>>,
    captures: OnceLock<(usize, usize)>,
}
pub(super) struct Pattern<'a> {
    pub text: &'a str,
    pub offset: usize,
    pub extended: bool,
    pub delimiters: [char; 2],
    pub(super) interpolations: Vec<(usize, usize)>,
    analysis: Arc<Analysis>,
}
impl<'a> Pattern<'a> {
    pub fn new(c: &Context<'a>, n: Node<'_>) -> Option<Self> {
        let raw = c.text(n);
        let open = if raw.starts_with("%r") { 2 } else { 0 };
        let delimiter = raw[open..].chars().next()?;
        let close = options::closing(delimiter);
        let end = raw.rfind(close)?;
        Some(Self::build(c, raw, n, open, end, [delimiter, close]))
    }
    fn build(
        c: &Context<'a>,
        raw: &'a str,
        n: Node<'_>,
        open: usize,
        end: usize,
        delimiters: [char; 2],
    ) -> Self {
        let begin = open + delimiters[0].len_utf8();
        let offset = n.start_byte() + begin;
        Self {
            text: &raw[begin..end],
            offset,
            extended: raw[end + delimiters[1].len_utf8()..].contains('x'),
            delimiters,
            interpolations: interpolations(n, offset),
            analysis: c.source.cached(n.start_byte(), Analysis::default),
        }
    }
    pub(super) fn captures(&self) -> Option<(usize, usize)> {
        self.interpolations.is_empty().then(|| {
            *self
                .analysis
                .captures
                .get_or_init(|| scanner::captures(self.text, self.extended, &self.interpolations))
        })
    }
    pub(super) fn tokens(&self) -> &[scanner::Token] {
        self.analysis
            .tokens
            .get_or_init(|| scanner::scan(self.text, self.extended, &self.interpolations))
    }
    pub fn escapes(&self) -> Vec<(usize, bool)> {
        self.tokens()
            .iter()
            .filter(|t| self.text.as_bytes()[t.start] == b'\\')
            .map(|t| (t.start, t.inside))
            .collect()
    }
    pub fn sets(&self) -> Vec<(usize, usize, bool)> {
        let mut stack = vec![];
        let mut out = vec![];
        for token in self.tokens() {
            match self.text.as_bytes()[token.start] {
                b'[' => stack.push((token.start, token.extended)),
                b']' => self.close_set(&mut stack, &mut out, token.start, token.end),
                _ => {}
            }
        }
        out
    }
    fn close_set(
        &self,
        stack: &mut Vec<(usize, bool)>,
        out: &mut Vec<(usize, usize, bool)>,
        start: usize,
        end: usize,
    ) {
        if let Some((open, extended)) = stack.pop() {
            if ordinary_set(&self.text[open + 1..start]) {
                out.push((open, end, extended));
            }
        }
    }
}
fn interpolations(n: Node<'_>, offset: usize) -> Vec<(usize, usize)> {
    kids(n)
        .into_iter()
        .filter(|d| d.kind() == "interpolation")
        .map(|i| (i.start_byte() - offset, i.end_byte() - offset))
        .collect()
}
fn ordinary_set(text: &str) -> bool {
    !text.starts_with(']')
        && !text.contains("&&")
        && !text.starts_with(':')
        && !text.starts_with('.')
        && !text.starts_with('=')
}
