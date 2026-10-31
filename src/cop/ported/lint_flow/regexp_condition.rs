use super::super::context::*;
use tree_sitter::Node;
pub(super) fn regexp_condition(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let mut expr = n;
    while let Some(p) = expr.parent() {
        match p.kind() {
            "parenthesized_statements"|"unary"=>expr=p,
            "binary" if ["&&","||","and","or"].contains(&operator(c,p))=>expr=p,
            "if"|"unless"|"if_modifier"|"unless_modifier"|"while"|"until"|"while_modifier"|"until_modifier"|"conditional" if field(p,"condition")==Some(expr)=>return vec![Finding::new(n,"Do not use regexp literal as a condition. The regexp literal matches `$_` implicitly.").replace(n,format!("{} =~ $_",c.text(n)))],
            _=>return vec![],
        }
    }
    vec![]
}
