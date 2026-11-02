use super::super::context::*;
use tree_sitter::Node;
mod rewrite;
use rewrite::{finding, replace, wrap_literal};
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
    if a.iter()
        .all(|a| matches!(a.kind(), "pair" | "hash_splat_argument"))
        && !a.is_empty()
    {
        return vec![wrap_literal(c, literal_pairs(c, n, &a), n)];
    }
    vec![wrap_literal(c, multi_argument(c, n, &a), n)]
}
fn hash_arguments<'n>(c: &Context<'_>, n: Node<'n>) -> Option<Vec<Node<'n>>> {
    if n.kind() == "element_reference" {
        return index_arguments(c, n);
    }
    (c.method(n) == "[]" && ["Hash", "::Hash"].contains(&c.receiver(n))).then(|| semantic_args(n))
}
fn index_arguments<'n>(c: &Context<'_>, n: Node<'n>) -> Option<Vec<Node<'n>>> {
    let obj = field(n, "object")?;
    root_constant(c, obj, "Hash").then(|| {
        kids(n)
            .into_iter()
            .filter(|a| *a != obj && Some(*a) != block(n))
            .collect()
    })
}
fn literal_pairs(c: &Context<'_>, n: Node<'_>, a: &[Node<'_>]) -> Finding {
    if block(n).is_some() {
        return finding(c, n, LITERAL_HASH);
    }
    replace(c, n, LITERAL_HASH, {
        format!(
            "{{{}}}",
            a.iter().map(|a| c.text(*a)).collect::<Vec<_>>().join(", ")
        )
    })
}
fn multi_argument(c: &Context<'_>, n: Node<'_>, a: &[Node<'_>]) -> Finding {
    if a.len() % 2 != 0
        || block(n).is_some()
        || a.iter().any(|a| {
            matches!(
                a.kind(),
                "block_argument" | "splat_argument" | "hash_splat_argument"
            )
        })
    {
        return finding(c, n, LITERAL_PAIRS);
    }
    replace(c, n, LITERAL_PAIRS, {
        format!(
            "{{{}}}",
            a.chunks(2)
                .map(|p| format!("{} => {}", c.text(p[0]), c.text(p[1])))
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}
fn hash_single(c: &Context<'_>, n: Node<'_>, a: Node<'_>) -> Vec<Finding> {
    if a.kind() == "splat_argument" {
        return splat(c, n);
    }
    if a.kind() == "hash" {
        return vec![finding(c, n, LITERAL_HASH)];
    }
    if matches!(a.kind(), "pair" | "hash_splat_argument") {
        return vec![literal_pairs(c, n, &[a])];
    }
    if zip_without_arguments(c, a) {
        return vec![zip(c, n, a)];
    }
    vec![replace(c, n, TO_H, format!("{}.to_h", to_h_receiver(c, a)))]
}
fn zip_without_arguments(c: &Context<'_>, n: Node<'_>) -> bool {
    c.method(n) == "zip"
        && !(n.kind() == "identifier" && local_identifier(c, n))
        && semantic_args(n).is_empty()
        && block(n).is_none()
        && !safe(c, n)
}
fn splat(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.config.get_bool("AllowSplatArgument", true) {
        vec![]
    } else {
        vec![finding(c, n, "Prefer array_of_pairs.to_h to Hash[*array].")]
    }
}
fn zip(c: &Context<'_>, n: Node<'_>, a: Node<'_>) -> Finding {
    if std::str::from_utf8(&c.source.as_bytes()[a.byte_range()]).is_err() {
        return finding(c, n, TO_H);
    }
    finding(c, n, TO_H).replace_with(c, a, || {
        let text = c.text(a);
        if text.ends_with(')') {
            format!("{}[])", &text[..text.len() - 1])
        } else {
            format!("{text}([])")
        }
    })
}
fn to_h_receiver(c: &Context<'_>, a: Node<'_>) -> String {
    let text = c.text(a);
    if a.kind() == "binary" || is_call(a) && !semantic_args(a).is_empty() && !text.ends_with(')') {
        format!("({text})")
    } else {
        text.into()
    }
}
