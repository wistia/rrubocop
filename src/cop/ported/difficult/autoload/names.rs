use super::super::super::context::*;
use super::inflections;
use tree_sitter::Node;

pub(super) fn defined(
    c: &Context<'_>,
    n: Node<'_>,
    acronyms: &inflections::Acronyms,
) -> Option<String> {
    if matches!(n.kind(), "class" | "module") {
        Some(join(
            parent(c, n).as_deref(),
            c.text(field(n, "name")?).trim_start_matches("::"),
        ))
    } else {
        assigned(c, n, acronyms)
    }
}
fn assigned(c: &Context<'_>, n: Node<'_>, acronyms: &inflections::Acronyms) -> Option<String> {
    let left = field(n, "left").unwrap_or(n);
    Some(join(
        assignment_namespace(c, n, left).as_deref(),
        &inflections::normalized(c.text(field(left, "name").unwrap_or(left)), acronyms),
    ))
}
fn assignment_namespace(c: &Context<'_>, n: Node<'_>, left: Node<'_>) -> Option<String> {
    field(left, "scope")
        .map(|scope| c.text(scope).rsplit("::").next().unwrap_or("").to_owned())
        .or_else(|| parent(c, n))
}
fn join(namespace: Option<&str>, name: &str) -> String {
    namespace
        .filter(|s| !s.is_empty())
        .map_or_else(|| name.to_owned(), |p| format!("{p}::{name}"))
}
fn parent(c: &Context<'_>, n: Node<'_>) -> Option<String> {
    let mut parts = vec![];
    for ancestor in ancestors(n).collect::<Vec<_>>().into_iter().rev() {
        add_namespace(c, ancestor, &mut parts)?;
    }
    Some(scope_name(&parts))
}
fn scope_name(parts: &[String]) -> String {
    if parts.is_empty() {
        "Object".into()
    } else {
        parts.join("::")
    }
}
fn add_namespace(c: &Context<'_>, n: Node<'_>, parts: &mut Vec<String>) -> Option<()> {
    match n.kind() {
        "class" | "module" => parts.push(
            c.text(field(n, "name")?)
                .trim_start_matches("::")
                .to_owned(),
        ),
        "assignment" => assignment_parent(c, n, parts),
        "singleton_class" => return singleton(c, n, parts),
        "block" | "do_block" => return block_namespace(c, n, parts),
        _ => {}
    }
    Some(())
}
fn assignment_parent(c: &Context<'_>, n: Node<'_>, parts: &mut Vec<String>) {
    if !field(n, "right").is_some_and(|right| new_class_call(c, right)) {
        return;
    }
    if let Some(left) =
        field(n, "left").filter(|n| matches!(n.kind(), "constant" | "scope_resolution"))
    {
        parts.push(c.text(left).trim_start_matches("::").to_owned());
    }
}
fn singleton(c: &Context<'_>, n: Node<'_>, parts: &mut Vec<String>) -> Option<()> {
    let object = field(n, "value").or_else(|| kids(n).first().copied())?;
    let name = if c.text(object) == "self" {
        scope_name(parts)
    } else if matches!(object.kind(), "constant" | "scope_resolution") {
        c.text(object).into()
    } else {
        return None;
    };
    parts.push(format!("#<Class:{name}>"));
    Some(())
}
fn block_namespace(c: &Context<'_>, n: Node<'_>, parts: &mut Vec<String>) -> Option<()> {
    let call = n.parent()?;
    if c.method(call) == "class_eval" {
        return class_eval_namespace(c, call, parts);
    }
    new_class_block(c, call).then_some(())
}
fn class_eval_namespace(c: &Context<'_>, call: Node<'_>, parts: &mut Vec<String>) -> Option<()> {
    if let Some(receiver) = field(call, "receiver") {
        if !matches!(receiver.kind(), "constant" | "scope_resolution") {
            return None;
        }
        parts.push(c.text(receiver).trim_start_matches("::").into());
    }
    Some(())
}
fn new_class_block(c: &Context<'_>, call: Node<'_>) -> bool {
    new_class_call(c, call) && call.parent().is_some_and(|p| p.kind() == "assignment")
}
fn new_class_call(c: &Context<'_>, call: Node<'_>) -> bool {
    c.method(call) == "new"
        && matches!(
            c.receiver(call),
            "Class" | "Module" | "::Class" | "::Module"
        )
}
