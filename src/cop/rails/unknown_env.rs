use crate::cop::shared::{argument_nodes, call_method_name, call_receiver, node_text};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct UnknownEnv;

pub(super) fn regular(node: Node<'_>, source: &SourceFile) -> bool {
    node.kind() == "call"
        && node
            .child_by_field_name("operator")
            .is_none_or(|n| node_text(source, n) != "&.")
}
pub(super) fn rails_env(source: &SourceFile, node: Node<'_>) -> bool {
    regular(node, source)
        && node.child_by_field_name("block").is_none()
        && call_method_name(source, node) == Some(b"env")
        && argument_nodes(node).is_empty()
        && call_receiver(node).is_some_and(|n| match n.kind() {
            "constant" => node_text(source, n) == "Rails",
            "scope_resolution" if n.child_by_field_name("scope").is_none() => n
                .child_by_field_name("name")
                .is_some_and(|n| node_text(source, n) == "Rails"),
            _ => false,
        })
}
fn jaro_winkler(a: &str, b: &str) -> f64 {
    let (mut a, mut b) = (a.chars().collect::<Vec<_>>(), b.chars().collect::<Vec<_>>());
    if a.len() > b.len() {
        std::mem::swap(&mut a, &mut b);
    }
    let range = if b.len() > 3 { b.len() / 2 - 1 } else { 0 };
    let (mut left, mut right) = (vec![false; a.len()], vec![false; b.len()]);
    for i in 0..a.len() {
        for j in i.saturating_sub(range)..(i + range + 1).min(b.len()) {
            if !right[j] && a[i] == b[j] {
                left[i] = true;
                right[j] = true;
                break;
            }
        }
    }
    let m = left.iter().filter(|v| **v).count();
    if m == 0 {
        return 0.0;
    }
    let t = a
        .iter()
        .zip(&left)
        .filter_map(|(c, v)| v.then_some(c))
        .zip(b.iter().zip(&right).filter_map(|(c, v)| v.then_some(c)))
        .filter(|(a, b)| a != b)
        .count()
        / 2;
    let m = m as f64;
    let score = (m / a.len() as f64 + m / b.len() as f64 + (m - t as f64) / m) / 3.0;
    let prefix = a.iter().zip(&b).take(4).take_while(|(a, b)| a == b).count();
    if score > 0.7 {
        score + prefix as f64 * 0.1 * (1.0 - score)
    } else {
        score
    }
}
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<_> = b.chars().collect();
    let mut row: Vec<_> = (0..=b.len()).collect();
    for (i, a) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, b) in b.iter().enumerate() {
            let old = row[j + 1];
            row[j + 1] = (row[j] + 1)
                .min(old + 1)
                .min(diagonal + usize::from(a != *b));
            diagonal = old;
        }
    }
    row[b.len()]
}
fn normalize(s: &str) -> String {
    s.to_lowercase().replace('@', "")
}
fn suggestions<'a>(name: &str, envs: &'a [String]) -> Vec<&'a str> {
    let input = normalize(name);
    let len = input.chars().count();
    let mut words: Vec<_> = envs
        .iter()
        .filter(|w| {
            w.as_str() != name
                && jaro_winkler(&normalize(w), &input) >= if len > 3 { 0.834 } else { 0.77 }
        })
        .collect();
    words.sort_by(|a, b| jaro_winkler(a, &input).total_cmp(&jaro_winkler(b, &input)));
    words.reverse();
    let result: Vec<_> = words
        .iter()
        .filter(|w| distance(&normalize(w), &input) <= len.div_ceil(4))
        .map(|w| w.as_str())
        .collect();
    if !result.is_empty() {
        return result;
    }
    words
        .into_iter()
        .find(|w| {
            let word = normalize(w);
            distance(&word, &input) < len.min(word.chars().count())
        })
        .map(|w| vec![w.as_str()])
        .unwrap_or_default()
}
impl Cop for UnknownEnv {
    fn name(&self) -> &'static str {
        "Rails/UnknownEnv"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "binary"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut envs = match config.options.get("Environments") {
            None => vec!["development".into(), "test".into(), "production".into()],
            Some(v) => v
                .as_sequence()
                .map(|v| {
                    v.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        };
        if config.get_f64("TargetRailsVersion", 8.1) >= 7.1 {
            envs.push("local".into());
        }
        let mut offense = None;
        if regular(node, source) && !crate::cop::shared::is_setter_call(node) {
            if let (Some(receiver), Some(method)) =
                (call_receiver(node), node.child_by_field_name("method"))
            {
                let name = node_text(source, method);
                if rails_env(source, receiver)
                    && argument_nodes(node).is_empty()
                    && name.ends_with('?')
                    && !envs.contains(&name[..name.len() - 1].to_string())
                {
                    offense = Some((method, name));
                }
            }
        }
        let operands = if node.kind() == "binary" {
            let op = node
                .child_by_field_name("operator")
                .map(|n| node_text(source, n));
            if matches!(op.as_deref(), Some("==" | "===")) {
                node.child_by_field_name("left")
                    .zip(node.child_by_field_name("right"))
            } else {
                None
            }
        } else if regular(node, source)
            && matches!(call_method_name(source, node), Some(b"==" | b"==="))
        {
            let args = argument_nodes(node);
            if args.len() == 1 {
                call_receiver(node).map(|n| (n, args[0]))
            } else {
                None
            }
        } else {
            None
        };
        if let Some((a, b)) = operands {
            for (env, string) in [(a, b), (b, a)] {
                if rails_env(source, env)
                    && matches!(string.kind(), "string" | "character" | "heredoc_beginning")
                {
                    if let Some(parts) = crate::cop::ported::ruby_string_parts(source, string) {
                        if parts.len() == 1 && !envs.contains(&parts[0]) {
                            offense = Some((string, parts[0].clone()));
                        }
                    }
                }
            }
        }
        if let Some((range, name)) = offense {
            let name = name.strip_suffix('?').unwrap_or(&name);
            let similar = suggestions(name, &envs);
            let message = if similar.is_empty() {
                format!("Unknown environment `{name}`.")
            } else {
                format!(
                    "Unknown environment `{name}`. Did you mean `{}`?",
                    similar.join(", ")
                )
            };
            let (line, col) = source.offset_to_line_col(range.start_byte());
            let mut d = self.diagnostic(source, line, col, message);
            d.highlight_length =
                super::blank::highlight_length(source, range.start_byte(), range.end_byte());
            diagnostics.push(d);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_did_you_mean_order_and_duplicates() {
        let envs = ["development", "production", "test"].map(String::from);
        assert_eq!(suggestions("proudction", &envs), ["production"]);
        assert_eq!(suggestions("developpment", &envs), ["development"]);
        assert!(suggestions("something", &envs).is_empty());
        let envs = ["staging", "staging", "stage"].map(String::from);
        assert_eq!(
            suggestions("stagin", &envs),
            ["staging", "staging", "stage"]
        );
    }
}
