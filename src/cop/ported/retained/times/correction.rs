use super::*;
mod indent;
mod pattern;
mod scopes;
use indent::deindent;
use pattern::parameter_pattern;
pub(super) fn correct_body(
    c: &Context<'_>,
    n: Node<'_>,
    b: Node<'_>,
    body: Node<'_>,
    f: Finding,
) -> Finding {
    let parameter = block_parameter(c, b);
    if !safety::extraction(b, body, c)
        || reassigns_parameter(c, body, parameter)
        || !substitution_safe(c, body, parameter)
    {
        return f;
    }
    corrected_body(c, n, body, parameter, f)
}

fn corrected_body(
    c: &Context<'_>,
    n: Node<'_>,
    body: Node<'_>,
    parameter: &str,
    f: Finding,
) -> Finding {
    f.replace_with(c, n, || {
        deindent(
            &replace_parameter(c, c.text(body), parameter),
            n.start_position().column,
            body.start_position().column,
        )
    })
}
fn block_parameter<'a>(c: &Context<'a>, b: Node<'_>) -> &'a str {
    let ps = field(b, "parameters").map(kids).unwrap_or_default();
    if ps.len() == 1 && ps[0].kind() == "identifier" {
        c.text(ps[0])
    } else {
        ""
    }
}
fn reassigns_parameter(c: &Context<'_>, body: Node<'_>, parameter: &str) -> bool {
    !parameter.is_empty()
        && descendants(body).any(|n| {
            matches!(n.kind(), "assignment" | "operator_assignment")
                && field(n, "left").is_some_and(|l| assignment_contains(c, l, parameter))
        })
}
fn replace_parameter(c: &Context<'_>, text: &str, parameter: &str) -> String {
    if parameter.is_empty() {
        return text.into();
    }
    parameter_pattern(c, parameter)
        .0
        .replace_all(text, "0")
        .into_owned()
}

fn assignment_contains(c: &Context<'_>, n: Node<'_>, parameter: &str) -> bool {
    if n.kind() == "identifier" {
        return c.text(n) == parameter;
    }
    matches!(
        n.kind(),
        "left_assignment_list" | "destructured_left_assignment" | "rest_assignment"
    ) && kids(n)
        .into_iter()
        .any(|n| assignment_contains(c, n, parameter))
}

fn substitution_safe(c: &Context<'_>, body: Node<'_>, parameter: &str) -> bool {
    if parameter.is_empty() {
        return true;
    }
    !scopes::unsafe_context(c, body, parameter)
        && parameter_pattern(c, parameter)
            .0
            .find_iter(c.text(body))
            .count()
            == descendants(body)
                .filter(|n| c.text(*n) == parameter && reference(*n))
                .count()
}
fn reference(n: Node<'_>) -> bool {
    if n.kind() != "identifier" {
        return false;
    }
    let Some(p) = n.parent() else {
        return false;
    };
    !matches!(
        p.kind(),
        "method_parameters" | "block_parameters" | "lambda_parameters"
    ) && field(p, "method") != Some(n)
        && field(p, "name") != Some(n)
}
