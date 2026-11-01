mod heredoc;
mod safety;
use super::super::context::*;
use tree_sitter::Node;
pub(super) fn file_read(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    file_access(c, n, false)
}
pub(super) fn file_write(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    file_access(c, n, true)
}
fn file_access(c: &Context<'_>, n: Node<'_>, writing: bool) -> Vec<Finding> {
    let Some(mode) = file_mode(c, n, writing) else {
        return vec![];
    };
    let Some((target, content)) = file_body(c, n, writing) else {
        return vec![];
    };
    vec![file_finding(
        c,
        n,
        target,
        content,
        &access_method(mode, writing),
    )]
}
fn file_mode<'c>(c: &'c Context<'_>, n: Node<'_>, writing: bool) -> Option<&'c str> {
    if !file_open(c, n) {
        return None;
    }
    let a = args(n);
    if !argument_count(&a, writing) {
        return None;
    }
    let mode = a
        .get(1)
        .filter(|a| a.kind() == "string")
        .map(|a| c.value(*a))
        .unwrap_or("r");
    (valid_mode(mode, writing) && valid_options(&a)).then_some(mode)
}
fn valid_mode(mode: &str, writing: bool) -> bool {
    if writing {
        ["w", "wt", "wb", "w+", "w+t", "w+b"].contains(&mode)
    } else {
        ["r", "rt", "rb", "r+", "r+t", "r+b"].contains(&mode)
    }
}
fn valid_options(a: &[Node<'_>]) -> bool {
    a.iter()
        .skip(1)
        .all(|a| matches!(a.kind(), "string" | "block_argument"))
}
fn access_method(mode: &str, writing: bool) -> String {
    format!(
        "{}{}",
        if mode.ends_with('b') { "bin" } else { "" },
        if writing { "write" } else { "read" }
    )
}
fn file_finding(
    c: &Context<'_>,
    n: Node<'_>,
    target: Node<'_>,
    content: Option<Node<'_>>,
    method: &str,
) -> Finding {
    let finding = Finding::new(target, format!("Use `File.{method}`."));
    if content.is_some_and(|content| !safety::write(c, n, target, content)) {
        return finding;
    }
    finding.edit_with(c, method_node(n).start_byte(), target.end_byte(), || {
        let extra = content
            .map(|a| format!(", {}", c.text(a)))
            .unwrap_or_default();
        format!(
            "{method}({}{extra}){}",
            c.text(args(n)[0]),
            heredoc::suffix(c, n, target, content)
        )
    })
}
fn file_body<'a>(
    c: &Context<'_>,
    n: Node<'a>,
    writing: bool,
) -> Option<(Node<'a>, Option<Node<'a>>)> {
    if !writing && read_block_pass(c, n) {
        return Some((n, None));
    }
    let (target, call) = file_call(c, n)?;
    if c.method(call) != if writing { "write" } else { "read" } {
        return None;
    }
    call_content(call, writing).map(|content| (target, content))
}
fn read_block_pass(c: &Context<'_>, n: Node<'_>) -> bool {
    args(n)
        .iter()
        .any(|a| a.kind() == "block_argument" && c.text(*a) == "&:read")
}
fn file_call<'a>(c: &Context<'_>, n: Node<'a>) -> Option<(Node<'a>, Node<'a>)> {
    if let Some(b) = block(n) {
        return block_call(c, n, b);
    }
    let p = n.parent()?;
    (is_call(p) && field(p, "receiver") == Some(n)).then_some((p, p))
}
fn block_call<'a>(c: &Context<'_>, n: Node<'a>, b: Node<'a>) -> Option<(Node<'a>, Node<'a>)> {
    let p = field(b, "parameters").map(kids)?;
    let s = statements(body(b)?)
        .into_iter()
        .filter(|node| node.kind() != "heredoc_body")
        .collect::<Vec<_>>();
    (p.len() == 1 && s.len() == 1 && c.receiver(s[0]) == c.text(p[0])).then(|| (n, s[0]))
}
fn call_content(n: Node<'_>, writing: bool) -> Option<Option<Node<'_>>> {
    let a = args(n);
    if writing {
        if a.len() != 1 || a[0].kind() == "splat_argument" {
            return None;
        }
        Some(Some(a[0]))
    } else if a.is_empty() {
        Some(None)
    } else {
        None
    }
}

fn file_open(c: &Context<'_>, n: Node<'_>) -> bool {
    c.method(n) == "open" && ["File", "::File"].contains(&c.receiver(n))
}

fn argument_count(arguments: &[Node<'_>], writing: bool) -> bool {
    if writing {
        arguments.len() == 2
    } else {
        !arguments.is_empty()
    }
}
