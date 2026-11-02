use super::*;

pub(super) fn finding(c: &Context<'_>, n: Node<'_>, message: &str) -> Finding {
    Finding::range(n.start_byte(), replacement_end(c, n), message)
}

pub(super) fn replace(c: &Context<'_>, n: Node<'_>, message: &str, text: String) -> Finding {
    let end = replacement_end(c, n);
    let finding = finding(c, n, message);
    let source_bytes = c.source.as_bytes();
    if std::str::from_utf8(&source_bytes[n.start_byte()..end]).is_err() {
        return finding;
    }
    finding.edit_with(c, n.start_byte(), end, || text)
}

fn replacement_end(c: &Context<'_>, n: Node<'_>) -> usize {
    let bytes = &c.source.as_bytes()[n.start_byte()..send_end(n)];
    n.start_byte()
        + bytes
            .iter()
            .rposition(|byte| !byte.is_ascii_whitespace())
            .map_or(0, |index| index + 1)
}

pub(super) fn wrap_literal(c: &Context<'_>, finding: Finding, n: Node<'_>) -> Finding {
    if !literal_correction(&finding) {
        return finding;
    }
    let Some(arguments) = bare_arguments(c, n) else {
        return finding;
    };
    let Some(parent) = arguments.parent().filter(|p| is_call(*p)) else {
        return finding;
    };
    finding
        .also_edit_with(
            c,
            method_node(parent).end_byte(),
            arguments.start_byte(),
            || "(",
        )
        .also_edit_with(c, arguments.end_byte(), arguments.end_byte(), || ")")
}

fn literal_correction(finding: &Finding) -> bool {
    matches!(finding.message.as_str(), LITERAL_HASH | LITERAL_PAIRS) && finding.correctable
}
fn bare_arguments<'n>(c: &Context<'_>, n: Node<'n>) -> Option<Node<'n>> {
    n.parent().filter(|a| {
        a.kind() == "argument_list" && c.source.as_bytes().get(a.start_byte()) != Some(&b'(')
    })
}
