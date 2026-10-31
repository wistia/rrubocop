use super::super::context::*;
use tree_sitter::Node;
pub(super) fn class_equality(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some((left, right)) = comparison(c, n) else {
        return vec![];
    };
    if interpolated(right) || allowed_method(c, n) {
        return vec![];
    }
    let named = name_method(c, left);
    let Some(class) = class_call(c, left, named) else {
        return vec![];
    };
    vec![class_finding(n, class, class_argument(c, n, right, named))]
}
fn comparison<'n>(c: &Context<'_>, n: Node<'n>) -> Option<(Node<'n>, Node<'n>)> {
    if n.kind() == "binary" && operator(c, n) == "==" {
        field(n, "left").zip(field(n, "right"))
    } else if ["==", "eql?", "equal?"].contains(&c.method(n)) {
        field(n, "receiver").zip(args(n).first().copied())
    } else {
        None
    }
}
fn interpolated(n: Node<'_>) -> bool {
    descendants(n).iter().any(|d| d.kind() == "interpolation")
}
fn name_method(c: &Context<'_>, n: Node<'_>) -> bool {
    ["name", "to_s", "inspect"].contains(&c.method(n))
}
fn class_call<'n>(c: &Context<'_>, n: Node<'n>, named: bool) -> Option<Node<'n>> {
    (if named { field(n, "receiver") } else { Some(n) }).filter(|cl| c.method(*cl) == "class")
}
fn class_finding(n: Node<'_>, class: Node<'_>, argument: Option<String>) -> Finding {
    let suffix = argument
        .as_ref()
        .map(|a| format!("({a})"))
        .unwrap_or_default();
    let start = method_node(class).start_byte();
    let f = Finding::range(
        start,
        n.end_byte(),
        format!("Use `instance_of?{suffix}` instead of comparing classes."),
    );
    if argument.is_some() {
        f.edit(start, n.end_byte(), format!("instance_of?{suffix}"))
    } else {
        f
    }
}
fn allowed_method(c: &Context<'_>, n: Node<'_>) -> bool {
    let Some(scope) = ancestors(n)
        .into_iter()
        .find(|p| matches!(p.kind(), "method" | "singleton_method"))
    else {
        return false;
    };
    let name = c.method(scope);
    list(c, "AllowedMethods").contains(&name)
        || list(c, "AllowedPatterns")
            .iter()
            .any(|p| regex::Regex::new(p).is_ok_and(|r| r.is_match(name)))
}
fn list<'c>(c: &'c Context<'_>, key: &str) -> Vec<&'c str> {
    c.config
        .options
        .get(key)
        .and_then(|v| v.as_sequence())
        .map(|v| v.iter().filter_map(|s| s.as_str()).collect())
        .unwrap_or_default()
}
fn class_argument(c: &Context<'_>, n: Node<'_>, right: Node<'_>, named: bool) -> Option<String> {
    if !named {
        return Some(c.text(right).into());
    }
    if name_method(c, right) {
        return field(right, "receiver").map(|r| c.text(r).into());
    }
    if right.kind() == "string" {
        return Some(string_class(c, n, right));
    }
    if unknown_class(right) {
        None
    } else {
        Some(c.text(right).into())
    }
}
fn unknown_class(n: Node<'_>) -> bool {
    is_call(n)
        || matches!(
            n.kind(),
            "identifier" | "instance_variable" | "class_variable" | "global_variable"
        )
}
fn string_class(c: &Context<'_>, n: Node<'_>, right: Node<'_>) -> String {
    let prefix = if ancestors(n)
        .iter()
        .any(|p| matches!(p.kind(), "class" | "module"))
    {
        "::"
    } else {
        ""
    };
    format!("{prefix}{}", c.value(right))
}
