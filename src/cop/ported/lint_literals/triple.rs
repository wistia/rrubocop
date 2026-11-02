use super::super::context::*;
use tree_sitter::Node;
pub(super) fn triple(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if !triple_quoted(c, n) {
        return vec![];
    }
    vec![
        Finding::new(
            n,
            "Delimiting a string with multiple quotes has no effect, use a single quote instead.",
        )
        .replace_with(c, n, || replacement(c, n)),
    ]
}
fn triple_quoted(c: &Context<'_>, n: Node<'_>) -> bool {
    c.text(n)
        .chars()
        .take_while(|ch| matches!(ch, '\'' | '"'))
        .count()
        >= 3
        && kids(n).into_iter().any(|string| empty(c, string))
}
fn empty(c: &Context<'_>, n: Node<'_>) -> bool {
    c.ruby_static_string(n).as_deref() == Some("")
}
fn replacement(c: &Context<'_>, n: Node<'_>) -> String {
    let strings = kids(n);
    let all_empty = strings.iter().all(|s| empty(c, *s));
    let mut text = c.text(n).to_string();
    for (index, string) in strings.iter().enumerate().rev() {
        if redundant_empty(c, *string, all_empty, index) {
            remove_string(&mut text, n, *string);
        }
    }
    text
}

fn redundant_empty(c: &Context<'_>, n: Node<'_>, all_empty: bool, index: usize) -> bool {
    empty(c, n) && !(all_empty && index == 0)
}

fn remove_string(text: &mut String, chain: Node<'_>, string: Node<'_>) {
    text.replace_range(
        string.start_byte() - chain.start_byte()..string.end_byte() - chain.start_byte(),
        "",
    );
}

#[cfg(test)]
mod tests;
