use super::super::context::*;
use tree_sitter::Node;
mod location;
pub(super) fn check(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if !eligible(c, n) {
        return vec![];
    }
    inspect(c, n, &arguments(n))
}
fn inspect(c: &Context<'_>, n: Node<'_>, arguments: &[Node<'_>]) -> Vec<Finding> {
    let Some(code) = code_argument(c, arguments) else {
        return vec![];
    };
    let base = if c.method(n) == "eval" { 2 } else { 1 };
    let mut out = file(c, n, arguments, base).into_iter().collect::<Vec<_>>();
    out.extend(line(c, n, code, arguments, base));
    if !known_arity(arguments, base) {
        clear_edits(&mut out);
    }
    out
}
fn code_argument<'n>(c: &Context<'_>, arguments: &[Node<'n>]) -> Option<Node<'n>> {
    arguments.first().copied().filter(|n| literal_code(c, *n))
}

fn arguments(n: Node<'_>) -> Vec<Node<'_>> {
    args(n)
        .into_iter()
        .filter(|a| !matches!(a.kind(), "comment" | "heredoc_body" | "heredoc_end"))
        .collect()
}
fn literal_code(c: &Context<'_>, n: Node<'_>) -> bool {
    match n.kind() {
        "string" | "chained_string" | "character" => true,
        "heredoc_beginning" => !c
            .text(n)
            .trim_start_matches("<<")
            .trim_start_matches(['-', '~'])
            .starts_with('`'),
        "identifier" => c.text(n) == "__FILE__",
        _ => false,
    }
}
fn file(c: &Context<'_>, n: Node<'_>, arguments: &[Node<'_>], base: usize) -> Option<Finding> {
    arguments
        .get(base)
        .filter(|f| c.text(**f) != "__FILE__")
        .map(|file| file_finding(c, n, *file))
}
fn line(
    c: &Context<'_>,
    n: Node<'_>,
    code: Node<'_>,
    arguments: &[Node<'_>],
    base: usize,
) -> Option<Finding> {
    if arguments.get(base + 1).is_none() {
        return Some(missing(c, n, code, arguments, base));
    }
    let line = line_argument(c, arguments)?;
    let diff = location::difference(c, code, line);
    (!location::correct(c, line, diff)).then(|| line_finding(c, n, line, diff))
}
fn line_argument<'n>(c: &Context<'_>, arguments: &[Node<'n>]) -> Option<Node<'n>> {
    arguments
        .last()
        .copied()
        .filter(|line| !location::variable_line(c, *line))
}

fn clear_edits(findings: &mut [Finding]) {
    for finding in findings {
        finding.correctable = false;
        finding.edit = None;
        finding.additional.clear();
    }
}
fn known_arity(arguments: &[Node<'_>], base: usize) -> bool {
    arguments.len() <= base + 2
        && !arguments.iter().any(|a| {
            matches!(
                a.kind(),
                "splat_argument"
                    | "hash_splat_argument"
                    | "block_argument"
                    | "forward_argument"
                    | "pair"
                    | "hash"
            )
        })
}

fn eligible(c: &Context<'_>, n: Node<'_>) -> bool {
    matches!(
        c.method(n),
        "eval" | "class_eval" | "module_eval" | "instance_eval"
    ) && (c.method(n) != "eval" || matches!(c.receiver(n), "" | "Kernel" | "::Kernel"))
}
fn file_finding(c: &Context<'_>, call: Node<'_>, file: Node<'_>) -> Finding {
    let f = Finding::new(
        file,
        format!(
            "Incorrect file for `{}`; use `__FILE__` instead of `{}`.",
            c.method(call),
            c.text(file)
        ),
    );
    if location::replaceable(c, file) {
        f.replace_with(c, file, || "__FILE__")
    } else {
        f
    }
}
fn line_finding(c: &Context<'_>, call: Node<'_>, line: Node<'_>, diff: isize) -> Finding {
    let expected = location::expected(diff);
    let f = Finding::new(
        line,
        format!(
            "Incorrect line number for `{}`; use `{expected}` instead of `{}`.",
            c.method(call),
            c.text(line)
        ),
    );
    if location::replaceable(c, line) {
        f.replace_with(c, line, || expected)
    } else {
        f
    }
}
fn missing(
    c: &Context<'_>,
    n: Node<'_>,
    code: Node<'_>,
    arguments: &[Node<'_>],
    base: usize,
) -> Finding {
    let f = Finding::range(n.start_byte(), send_end(n), missing_message(c, n));
    if !missing_correctable(c, n, arguments, base) {
        return f;
    }
    let last = *arguments.last().unwrap();
    f.edit_with(c, last.end_byte(), last.end_byte(), || {
        missing_text(c, code, last, arguments.len() <= base)
    })
}
fn missing_message(c: &Context<'_>, n: Node<'_>) -> String {
    if c.method(n) == "eval" {
        "Pass a binding, `__FILE__`, and `__LINE__` to `eval`.".into()
    } else {
        format!("Pass `__FILE__` and `__LINE__` to `{}`.", c.method(n))
    }
}
fn missing_text(c: &Context<'_>, code: Node<'_>, last: Node<'_>, file_missing: bool) -> String {
    let file = if file_missing { "__FILE__, " } else { "" };
    format!(
        ", {file}{}",
        location::expected(location::difference(c, code, last))
    )
}

fn missing_correctable(c: &Context<'_>, n: Node<'_>, arguments: &[Node<'_>], base: usize) -> bool {
    (c.method(n) != "eval" || arguments.len() >= 2) && known_arity(arguments, base)
}
