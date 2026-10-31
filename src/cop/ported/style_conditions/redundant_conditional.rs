use super::super::context::*;
use super::branches;
use tree_sitter::Node;
pub(super) fn redundant_conditional(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some(replacement) = replacement(c, n) else {
        return vec![];
    };
    let msg = if n.kind() == "elsif" {
        format!("\n{replacement}")
    } else {
        replacement.clone()
    };
    vec![
        Finding::new(
            n,
            format!("This conditional expression can just be replaced by `{msg}`."),
        )
        .replace_with(c, n, || replacement),
    ]
}
fn replacement(c: &Context<'_>, n: Node<'_>) -> Option<String> {
    let cond = field(n, "condition")?;
    if !comparison(c, cond) {
        return None;
    }
    let (yes, no) = branches(n);
    let expression = match (c.text(yes?), c.text(no?)) {
        ("true", "false") => c.text(cond).into(),
        ("false", "true") => format!("!({})", c.text(cond)),
        _ => return None,
    };
    Some(if n.kind() == "elsif" {
        format!(
            "else\n{}{}",
            " ".repeat(n.start_position().column + c.config.get_usize("IndentationWidth", 2)),
            expression
        )
    } else {
        expression
    })
}
fn comparison(c: &Context<'_>, n: Node<'_>) -> bool {
    ["==", "!=", "<", ">", "<=", ">=", "===", "=~", "!~", "<=>"].contains(
        &if n.kind() == "binary" {
            operator(c, n)
        } else {
            c.method(n)
        },
    )
}
