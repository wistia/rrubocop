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

enum OrderPattern {
    EverythingElse,
    Regex(Option<regex::Regex>),
    Name(String),
}
struct CompiledOrder {
    patterns: Vec<OrderPattern>,
    ranks: std::sync::Mutex<std::collections::HashMap<String, usize>>,
}
impl CompiledOrder {
    fn new(order: &[String]) -> Self {
        Self {
            patterns: order
                .iter()
                .map(|item| {
                    if item == "everything-else" {
                        OrderPattern::EverythingElse
                    } else if item.starts_with('/') && item.ends_with('/') && item.len() >= 2 {
                        OrderPattern::Regex(regex::Regex::new(&item[1..item.len() - 1]).ok())
                    } else {
                        OrderPattern::Name(item.clone())
                    }
                })
                .collect(),
            ranks: Default::default(),
        }
    }
    fn rank(&self, name: &str) -> usize {
        let mut ranks = self.ranks.lock().unwrap();
        if let Some(&rank) = ranks.get(name) {
            return rank;
        }
        *ranks.entry(name.to_owned()).or_insert_with(|| {
            let mut everything_else = self.patterns.len();
            for (index, pattern) in self.patterns.iter().enumerate() {
                match pattern {
                    OrderPattern::EverythingElse => everything_else = index,
                    OrderPattern::Regex(Some(regex)) if regex.is_match(name) => return index,
                    OrderPattern::Name(item) if item == name => return index,
                    _ => {}
                }
            }
            everything_else
        })
    }
}
pub fn correct_order(
    prev: &str,
    curr: &str,
    order: Option<&[String]>,
    source: &crate::parse::source::SourceFile,
) -> bool {
    let Some(order) = order else {
        return prev <= curr;
    };
    struct Orders(
        std::sync::Mutex<std::collections::HashMap<Vec<String>, std::sync::Arc<CompiledOrder>>>,
    );
    let orders = source.cached(0, || Orders(Default::default()));
    let compiled = {
        let mut orders = orders.0.lock().unwrap();
        if let Some(compiled) = orders.get(order) {
            compiled.clone()
        } else {
            let compiled = std::sync::Arc::new(CompiledOrder::new(order));
            orders.insert(order.to_vec(), compiled.clone());
            compiled
        }
    };
    let pi = compiled.rank(prev);
    let ci = compiled.rank(curr);
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

#[cfg(test)]
mod performance_tests {
    use super::*;
    #[test]
    fn compiled_order_preserves_first_match_invalid_patterns_and_config_isolation() {
        let source = crate::parse::source::SourceFile::from_bytes("test.rb", vec![]);
        let orders = [
            vec!["/^a/", "everything-else", "z"],
            vec!["/[invalid/", "z", "everything-else"],
            vec!["everything-else", "/.*/", "everything-else"],
            vec!["z", "/^a/"],
            vec![],
        ];
        let names = ["", "a", "aa", "b", "z", "é"];
        for order in orders {
            let order: Vec<String> = order.into_iter().map(str::to_owned).collect();
            let original_rank = |name: &str| {
                let mut fallback = order.len();
                for (index, item) in order.iter().enumerate() {
                    if item == "everything-else" {
                        fallback = index;
                    } else if item.starts_with('/') && item.ends_with('/') && item.len() >= 2 {
                        if regex::Regex::new(&item[1..item.len() - 1])
                            .is_ok_and(|re| re.is_match(name))
                        {
                            return index;
                        }
                    } else if item == name {
                        return index;
                    }
                }
                fallback
            };
            for previous in names {
                for current in names {
                    let expected =
                        (original_rank(previous), previous) <= (original_rank(current), current);
                    assert_eq!(
                        correct_order(previous, current, Some(&order), &source),
                        expected
                    );
                }
            }
        }
    }
}
