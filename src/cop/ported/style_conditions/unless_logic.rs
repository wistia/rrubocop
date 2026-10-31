use super::super::context::*;
use tree_sitter::Node;
pub(super) fn unless_logic(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some(cond) = field(n, "condition") else {
        return vec![];
    };
    let ops = logical_ops(c, cond);
    let msg = if c
        .config
        .get_str("EnforcedStyle", "forbid_mixed_logical_operators")
        == "forbid_logical_operators"
    {
        if !["&&", "||", "and", "or"].contains(&operator(c, cond)) {
            return vec![];
        }
        "Do not use any logical operator in an `unless`."
    } else {
        if !mixed(c, cond, &ops) {
            return vec![];
        }
        "Do not use mixed logical operators in an `unless`."
    };
    vec![Finding::new(n, msg)]
}

fn logical_ops<'c>(c: &'c Context<'_>, cond: Node<'_>) -> std::collections::HashSet<&'c str> {
    descendants(cond)
        .into_iter()
        .filter(|d| d.kind() == "binary")
        .map(|d| operator(c, d))
        .filter(|s| ["&&", "||", "and", "or"].contains(s))
        .collect()
}

fn mixed(c: &Context<'_>, cond: Node<'_>, ops: &std::collections::HashSet<&str>) -> bool {
    match operator(c, cond) {
        "&&" | "and" if ops.iter().any(|o| ["||", "or"].contains(o)) => true,
        "||" | "or" if ops.iter().any(|o| ["&&", "and"].contains(o)) => true,
        _ => ops.contains("&&") && ops.contains("and") || ops.contains("||") && ops.contains("or"),
    }
}
