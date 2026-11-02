use super::*;
mod indentation;

pub(super) fn value<'a>(c: &Context<'a>, n: Node<'_>) -> Option<Cow<'a, str>> {
    let quoted = quoted(c.text(n))?;
    let (start, end) = content_span(c, n)?;
    if !quoted && interpolated(root(n), start, end) {
        return None;
    }
    render(
        content(c, start, end)?,
        quoted,
        c.text(n).starts_with("<<~"),
    )
}

fn quoted(opener: &str) -> Option<bool> {
    let delimiter = opener.strip_prefix("<<")?.trim_start_matches(['-', '~']);
    (!delimiter.starts_with('`')).then(|| delimiter.starts_with('\''))
}

fn content<'a>(c: &Context<'a>, start: usize, end: usize) -> Option<&'a str> {
    std::str::from_utf8(c.source.as_bytes().get(start..end)?).ok()
}

fn root(n: Node<'_>) -> Node<'_> {
    ancestors(n).last().unwrap_or(n)
}

fn interpolated(root: Node<'_>, start: usize, end: usize) -> bool {
    descendants(root).any(|node| {
        node.kind() == "interpolation" && node.start_byte() >= start && node.start_byte() < end
    })
}

fn render(content: &str, quoted: bool, squiggly: bool) -> Option<Cow<'_, str>> {
    let content = if squiggly {
        Cow::Owned(indentation::strip(content))
    } else {
        Cow::Borrowed(content)
    };
    let content = if content.contains("\r\n") {
        Cow::Owned(content.replace("\r\n", "\n"))
    } else {
        content
    };
    if !single_part(&content, quoted) {
        return None;
    }
    if !quoted && content.contains('\\') {
        decode(&content, true, b'"').map(Cow::Owned)
    } else {
        Some(content)
    }
}

fn single_part(content: &str, quoted: bool) -> bool {
    let mut lines = content.split_inclusive('\n').peekable();
    while let Some(line) = lines.next() {
        if lines.peek().is_some() && (quoted || !continued(line)) {
            return false;
        }
    }
    !content.is_empty()
}

fn continued(line: &str) -> bool {
    line.strip_suffix('\n')
        .is_some_and(|line| line.bytes().rev().take_while(|byte| *byte == b'\\').count() % 2 == 1)
}

fn content_span(c: &Context<'_>, n: Node<'_>) -> Option<(usize, usize)> {
    let start = after_line(c, n.end_byte());
    let start = preceding(n).into_iter().try_fold(start, |start, opener| {
        span(c, opener, start).map(|(_, end)| after_line(c, end))
    })?;
    span(c, n, start)
}

fn preceding(n: Node<'_>) -> Vec<Node<'_>> {
    let mut pending = descendants(root(n))
        .filter(|node| {
            node.kind() == "heredoc_beginning"
                && node.start_position().row == n.start_position().row
                && node.start_byte() < n.start_byte()
        })
        .collect::<Vec<_>>();
    pending.sort_by_key(Node::start_byte);
    pending
}

fn span(c: &Context<'_>, n: Node<'_>, start: usize) -> Option<(usize, usize)> {
    let mut offset = start;
    while offset < c.source.as_bytes().len() {
        let end = after_line(c, offset);
        let line = content(c, offset, end)?;
        if terminates(c.text(n), line) {
            return Some((start, offset));
        }
        offset = end;
    }
    None
}

fn terminates(opener: &str, line: &str) -> bool {
    let line = line.trim_end_matches(['\r', '\n']);
    let line = if opener.starts_with("<<~") || opener.starts_with("<<-") {
        line.trim_start_matches([' ', '\t'])
    } else {
        line
    };
    line == opener
        .trim_start_matches("<<")
        .trim_start_matches(['~', '-'])
        .trim_matches(['\'', '"', '`'])
}

fn after_line(c: &Context<'_>, start: usize) -> usize {
    c.source.as_bytes()[start..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map_or(c.source.as_bytes().len(), |offset| start + offset + 1)
}

pub(super) fn end_offset(c: &Context<'_>, n: Node<'_>) -> Option<usize> {
    content_span(c, n).map(|(_, end)| end)
}
