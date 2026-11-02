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
        .replace_with(c, n, || replacement(c, n)),
    ]
}
fn replacement(c: &Context<'_>, n: Node<'_>) -> String {
    let strings = kids(n);
    let all_empty = strings.iter().all(|s| c.text(*s).len() == 2);
    let mut text = c.text(n).to_string();
    for (index, string) in strings.iter().enumerate().rev() {
        if redundant_empty(c, *string, all_empty, index) {
            remove_string(&mut text, n, *string);
        }
    }
    text
}

fn redundant_empty(c: &Context<'_>, n: Node<'_>, all_empty: bool, index: usize) -> bool {
    c.text(n).len() == 2 && !(all_empty && index == 0)
}

fn remove_string(text: &mut String, chain: Node<'_>, string: Node<'_>) {
    text.replace_range(
        string.start_byte() - chain.start_byte()..string.end_byte() - chain.start_byte(),
        "",
    );
}

#[cfg(test)]
mod tests;
