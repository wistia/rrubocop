use super::super::super::context::*;
use super::inflections;
use tree_sitter::Node;
mod head;
mod prefixes;
use prefixes::parent;

pub(super) fn defined(
    c: &Context<'_>,
    n: Node<'_>,
    acronyms: &inflections::Acronyms,
    limit: usize,
) -> Option<String> {
    if matches!(n.kind(), "class" | "module") {
        Some(join(
            parent(c, n, limit).as_ref().map(|p| p.head()),
            &constant_name(c, field(n, "name")?),
            limit,
        ))
    } else {
        assigned(c, n, acronyms, limit)
    }
}
fn assigned(
    c: &Context<'_>,
    n: Node<'_>,
    acronyms: &inflections::Acronyms,
    limit: usize,
) -> Option<String> {
    let left = field(n, "left").unwrap_or(n);
    let name = inflections::normalized(c.text(field(left, "name").unwrap_or(left)), acronyms);
    Some(assignment_head(c, n, left, &name, limit))
}
fn assignment_head(
    c: &Context<'_>,
    n: Node<'_>,
    left: Node<'_>,
    name: &str,
    limit: usize,
) -> String {
    if let Some(scope) = field(left, "scope") {
        return join(
            Some(constant_name(c, scope).rsplit("::").next().unwrap_or("")),
            name,
            limit,
        );
    }
    join(parent(c, n, limit).as_ref().map(|p| p.head()), name, limit)
}

fn join(namespace: Option<&str>, name: &str, limit: usize) -> String {
    head::join(namespace.filter(|s| !s.is_empty()), name, limit)
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
        "class" | "module" => parts.push(constant_name(c, field(n, "name")?).into_owned()),
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
        parts.push(constant_name(c, left).into_owned());
    }
}
fn singleton(c: &Context<'_>, n: Node<'_>, parts: &mut Vec<String>) -> Option<()> {
    let object = field(n, "value").or_else(|| kids(n).first().copied())?;
    let name = if c.text(object) == "self" {
        scope_name(parts)
    } else if matches!(object.kind(), "constant" | "scope_resolution") {
        constant_name(c, object).into_owned()
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
        parts.push(constant_name(c, receiver).into_owned());
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

fn constant_name<'a>(c: &Context<'a>, node: Node<'_>) -> std::borrow::Cow<'a, str> {
    c.constant_name(node)
        .unwrap_or_else(|| c.text(node).trim_start_matches("::").into())
}
