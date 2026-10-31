use super::super::context::*;
use tree_sitter::Node;
pub(super) fn top_level(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    statements(n).into_iter().filter(|s| !allowed(c,*s)).map(|s| Finding::new(s,"Top-level code in an autoloaded file runs at boot when eager loaded. Only define constants here; expose a `self.run` and invoke it via `bin/rails runner 'Klass.run'`.")).collect()
}
pub(super) fn allowed(c: &Context<'_>, n: Node<'_>) -> bool {
    match n.kind() {
        "class" | "module" => true,
        "assignment" => n
            .child_by_field_name("left")
            .is_some_and(|l| matches!(l.kind(), "constant" | "scope_resolution")),
        "call" => [
            "require",
            "require_relative",
            "require_dependency",
            "private_constant",
        ]
        .contains(&c.method(n)),
        "if" | "unless" | "if_modifier" | "unless_modifier" => {
            guard(c, n) || conditional_definitions(c, n)
        }
        _ => false,
    }
}
pub(super) fn guard(c: &Context<'_>, n: Node<'_>) -> bool {
    let Some(cond) = n.child_by_field_name("condition") else {
        return false;
    };
    if cond.kind() != "binary"
        || cond
            .child_by_field_name("operator")
            .is_none_or(|o| c.text(o) != "==")
    {
        return false;
    }
    kids(cond)
        .iter()
        .any(|s| ["$PROGRAM_NAME", "$0"].contains(&c.text(*s)))
}
pub(super) fn conditional_definitions(c: &Context<'_>, n: Node<'_>) -> bool {
    kids(n)
        .into_iter()
        .filter(|ch| Some(*ch) != n.child_by_field_name("condition"))
        .all(|b| {
            if b.kind() == "elsif" {
                conditional_definitions(c, b)
            } else {
                statements(b).into_iter().all(|s| allowed(c, s))
            }
        })
}
