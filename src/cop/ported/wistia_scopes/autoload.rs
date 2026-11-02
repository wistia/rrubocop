use super::super::context::*;
use tree_sitter::Node;
pub(super) fn top_level(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    ruby_statements(n).into_iter().filter(|s| !allowed(c,*s)).map(|s| Finding::new(s,"Top-level code in an autoloaded file runs at boot when eager loaded. Only define constants here; expose a `self.run` and invoke it via `bin/rails runner 'Klass.run'`.")).collect()
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
        "if" | "unless" | "if_modifier" | "unless_modifier" | "conditional" => {
            guard(c, n) || conditional_definitions(c, n)
        }
        _ => false,
    }
}
pub(super) fn guard(c: &Context<'_>, n: Node<'_>) -> bool {
    let Some(cond) = n.child_by_field_name("condition") else {
        return false;
    };
    equality_sides(c, cond).iter().any(|side| {
        side.kind() == "global_variable" && ["$PROGRAM_NAME", "$0"].contains(&c.text(*side))
    })
}

fn equality_sides<'n>(c: &Context<'_>, n: Node<'n>) -> Vec<Node<'n>> {
    if n.kind() == "binary" && operator(c, n) == "==" {
        kids(n)
    } else if is_call(n) && !safe(c, n) && c.method(n) == "==" {
        field(n, "receiver")
            .into_iter()
            .chain(args(n).into_iter().take(1))
            .collect()
    } else {
        vec![]
    }
}

pub(super) fn conditional_definitions(c: &Context<'_>, n: Node<'_>) -> bool {
    kids(n)
        .into_iter()
        .filter(|ch| Some(*ch) != n.child_by_field_name("condition"))
        .all(|b| {
            if b.kind() == "elsif" {
                conditional_definitions(c, b)
            } else {
                ruby_statements(b).into_iter().all(|s| allowed(c, s))
            }
        })
}

fn ruby_statements(n: Node<'_>) -> Vec<Node<'_>> {
    let nodes = statements(n);
    match nodes.as_slice() {
        [only] if only.kind() == "parenthesized_statements" => kids(*only),
        _ => nodes,
    }
}
