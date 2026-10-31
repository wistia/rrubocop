use super::super::context::*;
use tree_sitter::Node;
pub(super) fn triple(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let t = c.text(n);
    if !t.starts_with("\"\"\"") && !t.starts_with("'''") {
        return vec![];
    }
    vec![
        Finding::new(
            n,
            "Delimiting a string with multiple quotes has no effect, use a single quote instead.",
        )
        .replace_with(c, n, || replacement(c, &kids(n))),
    ]
}
fn replacement(c: &Context<'_>, strings: &[Node<'_>]) -> String {
    let mut kept_empty = false;
    let all_empty = strings.iter().all(|s| c.text(*s).len() == 2);
    strings
        .iter()
        .filter(|s| keep(c, **s, all_empty, &mut kept_empty))
        .map(|s| c.text(*s))
        .collect()
}
fn keep(c: &Context<'_>, n: Node<'_>, all_empty: bool, kept_empty: &mut bool) -> bool {
    if c.text(n).len() != 2 {
        return true;
    }
    if all_empty && !*kept_empty {
        *kept_empty = true;
        return true;
    }
    false
}
