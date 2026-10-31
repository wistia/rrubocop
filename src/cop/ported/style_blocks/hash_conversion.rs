use super::super::context::*;
use tree_sitter::Node;
const LITERAL_HASH: &str = "Prefer literal hash to Hash[key: value, ...].";
const LITERAL_PAIRS: &str = "Prefer literal hash to Hash[arg1, arg2, ...].";
const TO_H: &str = "Prefer ary.to_h to Hash[ary].";
pub(super) fn hash_conversion(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some(a) = hash_arguments(c, n) else {
        return vec![];
    };
    if a.len() == 1 {
        return hash_single(c, n, a[0])
            .into_iter()
            .map(|f| wrap_literal(c, f, n))
            .collect();
    }
    if a.iter().all(|a| a.kind() == "pair") && !a.is_empty() {
        return vec![wrap_literal(c, literal_pairs(c, n, &a), n)];
    }
    vec![wrap_literal(c, multi_argument(c, n, &a), n)]
}
fn hash_arguments<'n>(c: &Context<'_>, n: Node<'n>) -> Option<Vec<Node<'n>>> {
    if n.kind() == "element_reference" {
        return index_arguments(c, n);
    }
    (c.method(n) == "[]" && ["Hash", "::Hash"].contains(&c.receiver(n))).then(|| args(n))
}
fn index_arguments<'n>(c: &Context<'_>, n: Node<'n>) -> Option<Vec<Node<'n>>> {
    let obj = field(n, "object")?;
    root_constant(c, obj, "Hash").then(|| kids(n).into_iter().filter(|a| *a != obj).collect())
}
fn literal_pairs(c: &Context<'_>, n: Node<'_>, a: &[Node<'_>]) -> Finding {
    Finding::new(n, LITERAL_HASH).replace(
        n,
        format!(
            "{{{}}}",
            a.iter().map(|a| c.text(*a)).collect::<Vec<_>>().join(", ")
        ),
    )
}
fn multi_argument(c: &Context<'_>, n: Node<'_>, a: &[Node<'_>]) -> Finding {
    let finding = Finding::new(n, LITERAL_PAIRS);
    if a.len() % 2 != 0 {
        return finding;
    }
    finding.replace(
        n,
        format!(
            "{{{}}}",
            a.chunks(2)
                .map(|p| format!("{} => {}", c.text(p[0]), c.text(p[1])))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    )
}
fn hash_single(c: &Context<'_>, n: Node<'_>, a: Node<'_>) -> Vec<Finding> {
    if a.kind() == "splat_argument" {
        return splat(c, n);
    }
    if matches!(a.kind(), "hash" | "pair") {
        return vec![Finding::new(n, LITERAL_HASH).replace(n, format!("{{{}}}", c.text(a)))];
    }
    if c.method(a) == "zip" && args(a).is_empty() {
        return vec![zip(c, n, a)];
    }
    vec![Finding::new(n, TO_H).replace(n, format!("{}.to_h", to_h_receiver(c, a)))]
}
fn splat(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.config.get_bool("AllowSplatArgument", true) {
        vec![]
    } else {
        vec![Finding::new(
            n,
            "Prefer array_of_pairs.to_h to Hash[*array].",
        )]
    }
}
fn zip(c: &Context<'_>, n: Node<'_>, a: Node<'_>) -> Finding {
    let text = c.text(a);
    let replacement = if text.ends_with(')') {
        format!("{}[])", &text[..text.len() - 1])
    } else {
        format!("{text}([])")
    };
    Finding::new(n, TO_H).replace(a, replacement)
}
fn to_h_receiver(c: &Context<'_>, a: Node<'_>) -> String {
    let text = c.text(a);
    if a.kind() == "binary" || is_call(a) && !args(a).is_empty() && !text.ends_with(')') {
        format!("({text})")
    } else {
        text.into()
    }
}

fn wrap_literal(c: &Context<'_>, finding: Finding, n: Node<'_>) -> Finding {
    if !literal_correction(&finding) {
        return finding;
    }
    let Some(arguments) = bare_arguments(c, n) else {
        return finding;
    };
    let Some(parent) = arguments.parent().filter(|p| is_call(*p)) else {
        return finding;
    };
    finding
        .also_edit(method_node(parent).end_byte(), arguments.start_byte(), "(")
        .also_edit(parent.end_byte(), parent.end_byte(), ")")
}
fn literal_correction(finding: &Finding) -> bool {
    matches!(finding.message.as_str(), LITERAL_HASH | LITERAL_PAIRS) && finding.edit.is_some()
}
fn bare_arguments<'n>(c: &Context<'_>, n: Node<'n>) -> Option<Node<'n>> {
    n.parent()
        .filter(|a| a.kind() == "argument_list" && !c.text(*a).starts_with('('))
}
