use super::*;
pub(super) fn correct_body(
    c: &Context<'_>,
    n: Node<'_>,
    b: Node<'_>,
    body: Node<'_>,
    f: Finding,
) -> Finding {
    let parameter = block_parameter(c, b);
    if reassigns_parameter(c, body, parameter) || !substitution_safe(c, body, parameter) {
        return f;
    }
    let Some(text) = deindent(
        &replace_parameter(c.text(body), parameter),
        n.start_position().column,
        body.start_position().column,
    ) else {
        return f;
    };
    f.replace(n, text)
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
        && descendants(body).iter().any(|n| {
            matches!(n.kind(), "assignment" | "operator_assignment")
                && field(*n, "left").is_some_and(|l| assignment_contains(c, l, parameter))
        })
}
fn replace_parameter(text: &str, parameter: &str) -> String {
    if parameter.is_empty() {
        return text.into();
    }
    regex::Regex::new(&format!(r"\b{}\b", regex::escape(parameter)))
        .unwrap()
        .replace_all(text, "0")
        .into_owned()
}
fn deindent(text: &str, start: usize, end: usize) -> Option<String> {
    text.split('\n')
        .enumerate()
        .map(|(i, line)| {
            if i == 0 || line.is_empty() || end <= start || start > line.len() {
                return Some(line.to_owned());
            }
            Some(format!(
                "{}{}",
                line.get(..start)?,
                line.get(end.min(line.len())..)?
            ))
        })
        .collect::<Option<Vec<_>>>()
        .map(|lines| lines.join("\n"))
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
    regex::Regex::new(&format!(r"\b{}\b", regex::escape(parameter)))
        .unwrap()
        .find_iter(c.text(body))
        .count()
        == descendants(body)
            .into_iter()
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
