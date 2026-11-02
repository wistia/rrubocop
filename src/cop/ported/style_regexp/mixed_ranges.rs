use super::super::context::*;
use super::pattern::Pattern;
use std::collections::HashMap;
use tree_sitter::Node;

struct Atom {
    end: usize,
    character: char,
    literal: bool,
}

struct Ranges<'a> {
    pattern: Pattern<'a>,
    brackets: HashMap<usize, u8>,
    escapes: HashMap<usize, usize>,
}

pub(super) fn ranges(context: &Context<'_>, node: Node<'_>) -> Vec<(usize, usize)> {
    let Some(pattern) = Pattern::new(context, node) else {
        return Vec::new();
    };
    let mut brackets = HashMap::new();
    let mut escapes = HashMap::new();
    for token in pattern.tokens() {
        let byte = pattern.text.as_bytes()[token.start];
        if byte == b'\\' {
            escapes.insert(
                token.start,
                escape_end(pattern.text, token.start, token.end),
            );
        } else if matches!(byte, b'[' | b']') {
            brackets.insert(token.start, byte);
        }
    }
    Ranges {
        pattern,
        brackets,
        escapes,
    }
    .collect()
}

impl Ranges<'_> {
    fn collect(&self) -> Vec<(usize, usize)> {
        let mut result = Vec::new();
        let mut index = 0;
        let mut depth = 0usize;
        while index < self.pattern.text.len() {
            if let Some(bracket) = self.brackets.get(&index) {
                depth = if *bracket == b'[' {
                    depth + 1
                } else {
                    depth.saturating_sub(1)
                };
                index += 1;
                continue;
            }
            let Some(first) = self.atom(index) else {
                index += self.pattern.text[index..]
                    .chars()
                    .next()
                    .unwrap()
                    .len_utf8();
                continue;
            };
            if depth > 0 && self.pattern.text.as_bytes().get(first.end) == Some(&b'-') {
                if let Some(last) = self.atom(first.end + 1) {
                    if first.literal && last.literal && mixed(first.character, last.character) {
                        result.push((self.pattern.offset + index, self.pattern.offset + last.end));
                    }
                    index = last.end;
                    continue;
                }
            }
            index = first.end;
        }
        result
    }

    fn atom(&self, start: usize) -> Option<Atom> {
        if self.brackets.contains_key(&start) {
            return None;
        }
        if let Some((_, end)) = self
            .pattern
            .interpolations
            .iter()
            .find(|(begin, _)| *begin == start)
        {
            return Some(Atom {
                end: *end,
                character: '\0',
                literal: false,
            });
        }
        let character = self.pattern.text.get(start..)?.chars().next()?;
        Some(Atom {
            end: self
                .escapes
                .get(&start)
                .copied()
                .unwrap_or(start + character.len_utf8()),
            character,
            literal: !self.escapes.contains_key(&start),
        })
    }
}

fn mixed(first: char, last: char) -> bool {
    first.is_ascii_alphabetic()
        && last.is_ascii_alphabetic()
        && first.is_ascii_uppercase() != last.is_ascii_uppercase()
}

fn escape_end(text: &str, start: usize, initial: usize) -> usize {
    let bytes = text.as_bytes();
    let maximum = match bytes.get(start + 1) {
        Some(b'x') => 2,
        Some(b'u') if bytes.get(start + 2) != Some(&b'{') => 4,
        Some(b'0'..=b'7') => 3,
        _ => return initial,
    };
    let mut end = start
        + if matches!(bytes.get(start + 1), Some(b'0'..=b'7')) {
            1
        } else {
            2
        };
    for _ in 0..maximum {
        let Some(byte) = bytes.get(end) else {
            break;
        };
        let valid = if matches!(bytes.get(start + 1), Some(b'0'..=b'7')) {
            matches!(byte, b'0'..=b'7')
        } else {
            byte.is_ascii_hexdigit()
        };
        if !valid {
            break;
        }
        end += 1;
    }
    end.max(initial)
}
