use super::*;
use std::collections::HashMap;
type Spans = HashMap<usize, (usize, usize)>;

/// Tree-sitter defers heredoc bodies and can flatten nested body interpolations.
/// Physical spans associate each interpolation with its evaluated argument.
pub(super) struct Heredocs<'n> {
    interpolations: HashMap<usize, Vec<Node<'n>>>,
}
impl<'n> Heredocs<'n> {
    pub(super) fn new(c: &Context<'_>, root: Node<'n>) -> Self {
        let mut interpolations = HashMap::new();
        if c.source.as_bytes().windows(2).any(|pair| pair == b"<<") {
            let (openers, nodes) = relevant_nodes(root);
            let mut ranges = Ranges {
                c,
                openers,
                nodes,
                spans: HashMap::new(),
            };
            for opener in ranges.openers.clone() {
                ranges.span(opener);
            }
            interpolations = ranges.interpolations();
        }
        Self { interpolations }
    }
    pub(super) fn at(&self, opener: Node<'_>) -> Vec<Node<'n>> {
        self.interpolations
            .get(&opener.start_byte())
            .cloned()
            .unwrap_or_default()
    }
}
fn relevant_nodes(root: Node<'_>) -> (Vec<Node<'_>>, Vec<Node<'_>>) {
    let mut openers = Vec::new();
    let mut interpolations = Vec::new();
    for node in descendants(root) {
        match node.kind() {
            "heredoc_beginning" => openers.push(node),
            "interpolation" => interpolations.push(node),
            _ => {}
        }
    }
    (openers, interpolations)
}
struct Ranges<'a, 's, 'n> {
    c: &'a Context<'s>,
    openers: Vec<Node<'n>>,
    nodes: Vec<Node<'n>>,
    spans: Spans,
}
impl<'n> Ranges<'_, '_, 'n> {
    fn span(&mut self, opener: Node<'n>) -> (usize, usize) {
        if let Some(span) = self.spans.get(&opener.start_byte()) {
            return *span;
        }
        let start = self.content_start(opener);
        let end = self.content_end(opener, start);
        self.spans.insert(opener.start_byte(), (start, end));
        (start, end)
    }
    fn content_start(&mut self, opener: Node<'n>) -> usize {
        match self
            .openers
            .iter()
            .copied()
            .filter(|n| {
                n.start_byte() < opener.start_byte()
                    && n.start_position().row == opener.start_position().row
            })
            .max_by_key(|n| n.start_byte())
        {
            Some(n) => self.span(n).1,
            None => after_line(self.c, opener.end_byte()),
        }
    }
    fn content_end(&mut self, opener: Node<'n>, mut offset: usize) -> usize {
        while offset < self.c.source.as_bytes().len() {
            let end = after_line(self.c, offset);
            if !self.inside_interpolation(offset, end) && terminates(self.c, opener, offset, end) {
                return end;
            }
            offset = self.after_nested(offset, end);
        }
        offset
    }
    fn inside_interpolation(&self, start: usize, end: usize) -> bool {
        self.nodes
            .iter()
            .any(|n| n.start_byte() < start && end <= n.end_byte())
    }
    fn after_nested(&mut self, start: usize, end: usize) -> usize {
        self.openers
            .iter()
            .copied()
            .filter(|n| start <= n.start_byte() && n.start_byte() < end)
            .max_by_key(|n| n.start_byte())
            .map_or(end, |n| self.span(n).1)
    }
    fn owner(&self, node: Node<'_>) -> Option<usize> {
        self.openers
            .iter()
            .filter(|opener| {
                let (start, end) = self.spans[&opener.start_byte()];
                start <= node.start_byte() && node.end_byte() <= end
            })
            .max_by_key(|opener| self.spans[&opener.start_byte()].0)
            .map(|n| n.start_byte())
    }
    fn interpolations(&self) -> HashMap<usize, Vec<Node<'n>>> {
        let mut result: HashMap<usize, Vec<Node<'n>>> = HashMap::new();
        for node in &self.nodes {
            let Some(owner) = self.owner(*node) else {
                continue;
            };
            if !ancestors(*node)
                .any(|p| p.kind() == "interpolation" && self.owner(p) == Some(owner))
            {
                result.entry(owner).or_default().push(*node);
            }
        }
        result
    }
}
fn after_line(c: &Context<'_>, offset: usize) -> usize {
    c.source.as_bytes()[offset..]
        .iter()
        .position(|b| *b == b'\n')
        .map_or(c.source.as_bytes().len(), |end| offset + end + 1)
}
fn terminates(c: &Context<'_>, opener: Node<'_>, start: usize, end: usize) -> bool {
    let opening = c.text(opener);
    let delimiter = opening
        .trim_start_matches("<<")
        .trim_start_matches(['~', '-'])
        .trim_matches(['\'', '"', '`']);
    let line = std::str::from_utf8(&c.source.content[start..end])
        .unwrap_or("")
        .trim_end_matches(['\r', '\n']);
    let line = if opening.starts_with("<<~") || opening.starts_with("<<-") {
        line.trim_start_matches([' ', '\t'])
    } else {
        line
    };
    line == delimiter
}
