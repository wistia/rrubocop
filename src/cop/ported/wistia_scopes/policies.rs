use super::super::context::*;
use tree_sitter::Node;
pub(super) fn is_policy(c: &Context<'_>, n: Node<'_>) -> bool {
    n.kind() == "class"
        && n.child_by_field_name("name")
            .is_some_and(|a| c.text(a).ends_with("Policy"))
}
pub(super) fn no_show(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.method(n) != "show?" {
        return vec![];
    }
    if super::super::custom_index::index(c, n).parent_policy_disabled(c, n) != Some(false) {
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
    let index = super::super::custom_index::index(c, n);
    if index.policy_scope(c, n) != index.policy_disabled(c, n) {
        return vec![];
    }
    vec![Finding::new(
        n,
        "Policies must define a scope that inherits from `ApplicationPolicy::Scope` (or `JwtPolicy::Scope`)\nOR explicitly disable scoping with `disable_scoping!`\n",
    )]
}
