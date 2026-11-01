use super::super::context::*;
use super::arguments as args;
use tree_sitter::Node;
mod safety;
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
    let f = Finding::new(n, format!("Put `{method}` mixins in separate statements."));
    if !safety::separated(n) {
        return f;
    }
    let a = args(n);
    let text = c.render_edit(|| {
        let indent = " ".repeat(n.start_position().column);
        a.iter()
            .rev()
            .map(|a| format!("{method} {}", c.text(*a)))
            .collect::<Vec<_>>()
            .join(&format!("\n{indent}"))
    });
    f.replace_with(c, n, || text)
}
fn grouped(c: &Context<'_>, mixins: &[Node<'_>], method: &str) -> Vec<Finding> {
    if mixins.len() < 2 {
        return vec![];
    }
    let replacement = grouped_replacement(c, mixins, method);
    mixins
        .iter()
        .filter(|n| has_arguments(**n))
        .map(|n| grouped_finding(c, mixins, method, replacement.as_deref(), *n))
        .collect()
}
fn has_arguments(n: Node<'_>) -> bool {
    !args(n).is_empty()
}
fn grouped_replacement(c: &Context<'_>, mixins: &[Node<'_>], method: &str) -> Option<String> {
    safety::grouped(c, mixins).then(|| c.render_edit(|| grouped_source(c, mixins, method)))
}

fn grouped_finding(
    c: &Context<'_>,
    mixins: &[Node<'_>],
    method: &str,
    replacement: Option<&str>,
    n: Node<'_>,
) -> Finding {
    let f = Finding::new(n, format!("Put `{method}` mixins in a single statement."));
    let Some(replacement) = replacement else {
        return f;
    };
    f.edit_with(
        c,
        mixins[0].start_byte(),
        mixins.last().unwrap().end_byte(),
        || replacement,
    )
}

fn grouped_source(c: &Context<'_>, mixins: &[Node<'_>], method: &str) -> String {
    let gaps = mixins
        .windows(2)
        .map(|pair| {
            let bytes = &c.source.as_bytes()[pair[0].end_byte()..pair[1].start_byte()];
            if bytes.iter().all(u8::is_ascii_whitespace) {
                ""
            } else {
                std::str::from_utf8(bytes).unwrap()
            }
        })
        .collect::<String>();
    format!("{method} {}{gaps}", mixin_names(c, mixins))
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
