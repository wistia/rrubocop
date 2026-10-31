use super::super::context::*;
use tree_sitter::Node;
fn end_token(n: Node<'_>) -> Option<Node<'_>> {
    let mut cur = n.walk();
    n.children(&mut cur).find(|ch| ch.kind() == "end")
}
fn statements_body(n: Node<'_>) -> Vec<Node<'_>> {
    field(n, "body").map(statements).unwrap_or_default()
}
pub(super) fn end(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some(e) = end_token(n) else {
        return vec![];
    };
    if n.start_position().row == e.start_position().row
        || !statements_body(n)
            .last()
            .is_some_and(|b| b.end_position().row == e.start_position().row)
    {
        return vec![];
    }
    vec![
        Finding::new(
            e,
            "Place the end statement of a multi-line method on its own line.",
        )
        .edit_with(c, e.start_byte(), e.start_byte(), || {
            format!("\n{}", " ".repeat(n.start_position().column))
        }),
    ]
}
pub(super) fn body(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some(first) = trailing_body(n) else {
        return vec![];
    };
    let indent = n.start_position().column + c.config.get_usize("IndentationWidth", 2);
    let f = Finding::new(
        first,
        "Place the first line of a multi-line method definition's body on its own line.",
    )
    .edit_with(c, first.start_byte(), first.start_byte(), || {
        format!("\n{}", " ".repeat(indent))
    });
    vec![move_comment(c, n, remove_semicolon(c, n, first, f))]
}
fn trailing_body(n: Node<'_>) -> Option<Node<'_>> {
    if end_token(n).is_none() || n.start_position().row == n.end_position().row {
        return None;
    }
    statements_body(n)
        .first()
        .copied()
        .filter(|first| first.start_position().row == n.start_position().row)
}
fn remove_semicolon(c: &Context<'_>, n: Node<'_>, first: Node<'_>, f: Finding) -> Finding {
    let mut cursor = n.walk();
    if let Some(token) = n
        .children(&mut cursor)
        .find(|ch| ch.kind() == ";" && ch.start_byte() < first.start_byte())
    {
        f.also_edit_with(c, token.start_byte(), token.end_byte(), || "")
    } else {
        f
    }
}

fn move_comment(c: &Context<'_>, n: Node<'_>, f: Finding) -> Finding {
    let Some(comment) = descendants(n)
        .find(|ch| ch.kind() == "comment" && ch.start_position().row == n.start_position().row)
    else {
        return f;
    };
    f.also_edit_with(c, n.start_byte(), n.start_byte(), || {
        format!(
            "{}\n{}",
            c.text(comment),
            " ".repeat(n.start_position().column)
        )
    })
    .also_edit_with(c, comment.start_byte(), comment.end_byte(), || "")
}
