use super::super::context::*;
use tree_sitter::Node;
pub(super) fn is_policy(c: &Context<'_>, n: Node<'_>) -> bool {
    n.kind() == "class"
        && n.child_by_field_name("name")
            .is_some_and(|a| c.text(a).ends_with("Policy"))
}
pub(super) fn disables_scope(c: &Context<'_>, n: Node<'_>) -> bool {
    descendants(n).any(|d| is_call(d) && c.method(d) == "disable_scoping!")
}
pub(super) fn no_show(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.method(n) != "show?" {
        return vec![];
    }
    let Some(policy) = ancestors(n).find(|p| is_policy(c, *p)) else {
        return vec![];
    };
    if disables_scope(c, policy) {
        return vec![];
    }
    vec![Finding::new(
        n,
        "Do not define a `show?` method in policies, `show?` logic should be encapsulated\nby the policy's scope unless the policy explicity disables scoping with `disable_scoping!`.\n",
    )]
}
pub(super) fn superclass<'a>(c: &'a Context<'_>, n: Node<'_>) -> &'a str {
    n.child_by_field_name("superclass")
        .and_then(|s| s.named_child(0))
        .map(|s| c.text(s).trim_start_matches("::"))
        .unwrap_or("")
}
pub(super) fn policy_scope(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if !is_policy(c, n) {
        return vec![];
    }
    if descendants(n).any(|d| {
        d.kind() == "class"
            && matches!(
                superclass(c, d),
                "ApplicationPolicy::Scope" | "JwtPolicy::Scope"
            )
    }) != disables_scope(c, n)
    {
        return vec![];
    }
    vec![Finding::new(
        n,
        "Policies must define a scope that inherits from `ApplicationPolicy::Scope` (or `JwtPolicy::Scope`)\nOR explicitly disable scoping with `disable_scoping!`\n",
    )]
}
