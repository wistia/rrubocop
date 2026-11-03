use super::{escape_token, options};
#[derive(Clone, Copy)]
pub(super) struct Token {
    pub start: usize,
    pub end: usize,
    pub inside: bool,
    pub extended: bool,
}
struct Class {
    first: usize,
}
struct Scanner<'a> {
    text: &'a str,
    interpolations: &'a [(usize, usize)],
    classes: Vec<Class>,
    groups: Vec<bool>,
    extended: bool,
    tokens: Vec<Token>,
    captures: (usize, usize),
    named: Vec<(usize, usize)>,
    emit_tokens: bool,
}
pub(super) fn scan(text: &str, extended: bool, interpolations: &[(usize, usize)]) -> Vec<Token> {
    run(text, extended, interpolations, true).tokens
}
pub(super) fn named(
    text: &str,
    extended: bool,
    interpolations: &[(usize, usize)],
) -> Vec<(usize, usize)> {
    run(text, extended, interpolations, false).named
}
pub(super) fn captures(
    text: &str,
    extended: bool,
    interpolations: &[(usize, usize)],
) -> (usize, usize) {
    run(text, extended, interpolations, false).captures
}
fn run<'a>(
    text: &'a str,
    extended: bool,
    interpolations: &'a [(usize, usize)],
    emit_tokens: bool,
) -> Scanner<'a> {
    let mut scanner = Scanner {
        text,
        interpolations,
        classes: vec![],
        groups: vec![],
        extended,
        tokens: vec![],
        captures: (0, 0),
        named: Vec::new(),
        emit_tokens,
    };
    scanner.run();
    scanner
}

impl Scanner<'_> {
    fn run(&mut self) {
        let mut i = 0;
        while i < self.text.len() {
            i = self.step(i);
        }
    }
    fn step(&mut self, i: usize) -> usize {
        if let Some(end) = self.skipped(i) {
            return end;
        }
        match self.text.as_bytes()[i] {
            b'\\' => self.escape(i),
            b'[' => self.open_class(i),
            b']' if self.closes_class(i) => self.close_class(i),
            b'(' if self.classes.is_empty() => self.open_group(i),
            b')' if self.classes.is_empty() => self.close_group(i),
            _ => {
                i + self.text[i..]
                    .chars()
                    .next()
                    .map(char::len_utf8)
                    .unwrap_or(1)
            }
        }
    }
    fn skipped(&self, i: usize) -> Option<usize> {
        if let Some((_, end)) = self.interpolations.iter().find(|(start, _)| *start == i) {
            return Some(*end);
        }
        if self.extended && self.classes.is_empty() && self.text.as_bytes()[i] == b'#' {
            return Some(i + self.text[i..].find('\n').unwrap_or(self.text.len() - i));
        }
        None
    }
    fn push(&mut self, start: usize, end: usize, inside: bool) {
        if !self.emit_tokens {
            return;
        }
        self.tokens.push(Token {
            start,
            end,
            inside,
            extended: self.extended,
        });
    }
    fn escape(&mut self, i: usize) -> usize {
        let end = escape_token::end(self.text, i);
        self.push(i, end, !self.classes.is_empty());
        end
    }
    fn open_class(&mut self, i: usize) -> usize {
        let first = i + 1 + usize::from(self.text.as_bytes().get(i + 1) == Some(&b'^'));
        self.classes.push(Class { first });
        self.push(i, i + 1, true);
        i + 1
    }
    fn closes_class(&self, i: usize) -> bool {
        self.classes.last().is_some_and(|c| i > c.first)
    }
    fn close_class(&mut self, i: usize) -> usize {
        self.classes.pop();
        self.push(i, i + 1, true);
        i + 1
    }
    fn open_group(&mut self, i: usize) -> usize {
        if self.text[i..].starts_with("(?#") {
            return options::comment_end(self.text, i + 3);
        }
        if let Some((end, extended, scoped)) = options::scoped(self.text, i, self.extended) {
            if scoped {
                self.groups.push(self.extended);
            }
            self.extended = extended;
            return end;
        }
        self.count_group(i);
        self.groups.push(self.extended);
        if self.text[i..].starts_with("(?(") {
            return self.text[i + 3..]
                .find(')')
                .map_or(self.text.len(), |end| i + 4 + end);
        }
        i + 1
    }
    fn count_group(&mut self, i: usize) {
        let text = &self.text[i..];
        if text.starts_with("(?'")
            || text.starts_with("(?<") && !text.starts_with("(?<=") && !text.starts_with("(?<!")
        {
            self.captures.0 += 1;
            let close = if text.starts_with("(?'") { '\'' } else { '>' };
            if let Some(end) = text[3..].find(close) {
                self.named.push((i + 3, i + 3 + end));
            }
        } else if !text.starts_with("(?") {
            self.captures.1 += 1;
        }
    }
    fn close_group(&mut self, i: usize) -> usize {
        if let Some(mode) = self.groups.pop() {
            self.extended = mode;
        }
        i + 1
    }
}

#[cfg(test)]
mod tests;
