use super::super::context::*;
use super::arguments as args;
use tree_sitter::Node;
pub(super) fn check(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some(body) = field(n, "body") else {
        return vec![];
    };
    let calls = kids(body)
        .into_iter()
        .filter(|ch| mixin_call(c, *ch))
        .collect::<Vec<_>>();
    ["include", "extend", "prepend"]
        .into_iter()
        .flat_map(|method| check_method(c, &calls, method))
        .collect()
}
fn mixin_call(c: &Context<'_>, n: Node<'_>) -> bool {
    is_call(n) && c.receiver(n).is_empty() && block(n).is_none()
}
fn check_method(c: &Context<'_>, calls: &[Node<'_>], method: &str) -> Vec<Finding> {
    let mixins = calls
        .iter()
        .filter(|ch| c.method(**ch) == method)
        .copied()
        .collect::<Vec<_>>();
    if c.config.get_str("EnforcedStyle", "separated") == "grouped" {
        grouped(c, &mixins, method)
    } else {
        separated_findings(c, &mixins, method)
    }
}
fn separated(c: &Context<'_>, n: Node<'_>, method: &str) -> Finding {
    let a = args(n);
    let text = c.render_edit(|| {
        let indent = " ".repeat(n.start_position().column);
        a.iter()
            .rev()
            .map(|a| format!("{method} {}", c.text(*a)))
            .collect::<Vec<_>>()
            .join(&format!("\n{indent}"))
    });
    Finding::new(n, format!("Put `{method}` mixins in separate statements.")).replace_with(
        c,
        n,
        || text,
    )
}
fn grouped(c: &Context<'_>, mixins: &[Node<'_>], method: &str) -> Vec<Finding> {
    if mixins.len() < 2 {
        return vec![];
    }
    let names = c.render_edit(|| mixin_names(c, mixins));
    mixins
        .iter()
        .enumerate()
        .filter(|(_, n)| !args(**n).is_empty())
        .map(|(i, n)| grouped_finding(c, mixins, method, &names, i, *n))
        .collect()
}
fn grouped_finding(
    c: &Context<'_>,
    mixins: &[Node<'_>],
    method: &str,
    names: &str,
    i: usize,
    n: Node<'_>,
) -> Finding {
    let f = Finding::new(n, format!("Put `{method}` mixins in a single statement."));
    if i == 0 {
        return f.replace_with(c, n, || format!("{method} {names}"));
    }
    f.edit_with(c, removal_start(c, mixins[i - 1], n), n.end_byte(), || "")
}
fn removal_start(c: &Context<'_>, previous: Node<'_>, n: Node<'_>) -> usize {
    let gap = &c.source.as_bytes()[previous.end_byte()..n.start_byte()];
    if gap.iter().all(u8::is_ascii_whitespace) {
        previous.end_byte()
    } else {
        n.start_byte()
    }
}

fn separated_findings(c: &Context<'_>, mixins: &[Node<'_>], method: &str) -> Vec<Finding> {
    mixins
        .iter()
        .filter(|n| args(**n).len() > 1)
        .map(|n| separated(c, *n, method))
        .collect()
}
fn mixin_names(c: &Context<'_>, mixins: &[Node<'_>]) -> String {
    mixins
        .iter()
        .rev()
        .flat_map(|m| args(*m))
        .map(|a| c.text(a))
        .collect::<Vec<_>>()
        .join(", ")
}
