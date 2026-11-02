use super::super::context::*;
use tree_sitter::Node;
fn single_block_arg(c: &Context<'_>, n: Node<'_>) -> bool {
    let Some(b) = block(n) else {
        return false;
    };
    if let Some(p) = field(b, "parameters") {
        let p = kids(p);
        return p.len() == 1 && p[0].kind() == "identifier";
    }
    let text = c.text(b);
    text.contains("_1") && !(2..=9).any(|i| text.contains(&format!("_{i}")))
}
pub(super) fn with_index(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    redundant_with(c, n, "index")
}
pub(super) fn with_object(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    redundant_with(c, n, "object")
}
fn redundant_with(c: &Context<'_>, n: Node<'_>, suffix: &str) -> Vec<Finding> {
    let each = format!("each_with_{suffix}");
    let with = format!("with_{suffix}");
    if !matches_method(c, n, &each, &with) || !eligible(c, n, suffix, &with) {
        return vec![];
    }
    let selector = method_node(n);
    let end = call_end(c, n, selector);
    let finding = Finding::range(
        selector.start_byte(),
        end,
        message(c.method(n) == each, &each, &with),
    );
    vec![correct(c, n, finding, suffix, &each, end)]
}
fn matches_method(c: &Context<'_>, n: Node<'_>, each: &str, with: &str) -> bool {
    c.method(n) == each || c.method(n) == with
}
fn eligible(c: &Context<'_>, n: Node<'_>, suffix: &str, with: &str) -> bool {
    if !single_block_arg(c, n) {
        return false;
    }
    if suffix == "object" {
        return semantic_args(n).len() == 1;
    }
    !c.receiver(n).is_empty()
        && (c.method(n) != with || field(n, "receiver").is_some_and(|r| !c.receiver(r).is_empty()))
}
fn call_end(c: &Context<'_>, n: Node<'_>, selector: Node<'_>) -> usize {
    c.source.as_bytes()[selector.start_byte()..send_end(n)]
        .iter()
        .rposition(|b| !b.is_ascii_whitespace())
        .map(|i| selector.start_byte() + i + 1)
        .unwrap_or(send_end(n))
}
fn message(each: bool, each_method: &str, with_method: &str) -> String {
    if each {
        format!("Use `each` instead of `{each_method}`.")
    } else {
        format!("Remove redundant `{with_method}`.")
    }
}
fn correct(
    c: &Context<'_>,
    n: Node<'_>,
    finding: Finding,
    suffix: &str,
    each: &str,
    end: usize,
) -> Finding {
    let selector = method_node(n);
    if c.method(n) == each {
        let end = if suffix == "index" {
            selector.end_byte()
        } else {
            end
        };
        finding.edit_with(c, selector.start_byte(), end, || "each")
    } else {
        finding.edit_with(c, operator_start(n, selector), end, || "")
    }
}
fn operator_start(n: Node<'_>, selector: Node<'_>) -> usize {
    field(n, "operator")
        .map(|d| d.start_byte())
        .unwrap_or(selector.start_byte() - 1)
}
