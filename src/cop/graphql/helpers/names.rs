//! Naming / order helpers for GraphQL Ruby DSL cops.

pub fn is_snake_case(name: &str) -> bool {
    let bytes = name.trim_start_matches('_').as_bytes();
    !bytes.is_empty()
        && bytes[0].is_ascii_lowercase()
        && bytes
            .iter()
            .all(|&b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

fn needs_underscore(chars: &[char], i: usize) -> bool {
    if i == 0 || !chars[i].is_uppercase() {
        return false;
    }
    let prev = chars[i - 1];
    prev.is_lowercase()
        || prev.is_ascii_digit()
        || (prev.is_uppercase() && chars.get(i + 1).is_some_and(|n| n.is_lowercase()))
}

fn push_cased(out: &mut String, c: char) {
    if c.is_uppercase() {
        out.extend(c.to_lowercase());
    } else {
        out.push(c);
    }
}

pub fn underscore(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::with_capacity(chars.len() + 4);
    for (i, &c) in chars.iter().enumerate() {
        if c == '-' {
            out.push('_');
            continue;
        }
        if needs_underscore(&chars, i) && !out.ends_with('_') {
            out.push('_');
        }
        push_cased(&mut out, c);
    }
    out
}

pub fn order_index(name: &str, order: &[String]) -> usize {
    let mut everything_else = order.len();
    for (i, item) in order.iter().enumerate() {
        if item == "everything-else" {
            everything_else = i;
        } else if item.starts_with('/') && item.ends_with('/') && item.len() >= 2 {
            let re = &item[1..item.len() - 1];
            if regex_simple_match(re, name) {
                return i;
            }
        } else if item == name {
            return i;
        }
    }
    everything_else
}

fn regex_simple_match(pat: &str, text: &str) -> bool {
    regex::Regex::new(pat)
        .map(|r| r.is_match(text))
        .unwrap_or(false)
}

pub fn correct_order(prev: &str, curr: &str, order: Option<&[String]>) -> bool {
    let Some(order) = order else {
        return prev <= curr;
    };
    let pi = order_index(prev, order);
    let ci = order_index(curr, order);
    if pi == ci { prev <= curr } else { pi < ci }
}

pub fn config_string_list(config: &crate::cop::CopConfig, key: &str) -> Vec<String> {
    config
        .options
        .get(key)
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

pub const CONFLICT_FIELD_NAMES: &[&str] = &[
    "context",
    "object",
    "raw_value",
    "class",
    "module",
    "def",
    "end",
    "if",
    "unless",
    "true",
    "false",
    "nil",
    "self",
    "and",
    "or",
    "not",
];

pub fn is_conflict_field_name(name: &str) -> bool {
    CONFLICT_FIELD_NAMES.contains(&name)
}
