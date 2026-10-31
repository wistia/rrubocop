use super::*;
pub(super) fn first_line(c: &Context<'_>, code: Node<'_>) -> usize {
    if code.kind() != "heredoc_beginning" {
        return code.start_position().row + 1;
    }
    let root = ancestors(code).last().unwrap_or(code);
    unique_body(c, code, &nodes(root, "heredoc_body"))
        .map_or_else(|| lexical_line(c, code, root), |body| body_line(c, body))
}
fn nodes<'n>(root: Node<'n>, kind: &str) -> Vec<Node<'n>> {
    let mut nodes = descendants(root)
        .filter(|n| n.kind() == kind)
        .collect::<Vec<_>>();
    nodes.sort_by_key(|n| n.start_byte());
    nodes
}
fn unique_body<'n>(c: &Context<'_>, code: Node<'_>, bodies: &[Node<'n>]) -> Option<Node<'n>> {
    let matching = bodies
        .iter()
        .filter(|b| body_delimiter(c, **b) == Some(delimiter(c, code)))
        .copied()
        .collect::<Vec<_>>();
    (matching.len() == 1).then(|| matching[0])
}
fn delimiter<'a>(c: &Context<'a>, code: Node<'_>) -> &'a str {
    c.text(code)
        .trim_start_matches("<<")
        .trim_start_matches(['~', '-'])
        .trim_matches(['\'', '"', '`'])
}
fn body_delimiter<'a>(c: &Context<'a>, body: Node<'_>) -> Option<&'a str> {
    kids(body)
        .into_iter()
        .find(|n| n.kind() == "heredoc_end")
        .map(|n| c.text(n).trim())
}
fn body_line(c: &Context<'_>, body: Node<'_>) -> usize {
    let tail = &c.source.as_bytes()[body.start_byte()..];
    let newline = if tail.starts_with(b"\r\n") {
        2
    } else {
        usize::from(tail.starts_with(b"\n"))
    };
    c.source.offset_to_line_col(body.start_byte() + newline).0
}

fn lexical_line(c: &Context<'_>, code: Node<'_>, root: Node<'_>) -> usize {
    c.source.offset_to_line_col(lexical_offset(c, code, root)).0
}
fn lexical_offset(c: &Context<'_>, code: Node<'_>, root: Node<'_>) -> usize {
    let start = after_line(c, code.end_byte());
    nodes(root, "heredoc_beginning")
        .into_iter()
        .filter(|n| {
            n.start_byte() < code.start_byte()
                && n.start_position().row == code.start_position().row
        })
        .try_fold(start, |offset, opener| after_terminator(c, opener, offset))
        .unwrap_or(start)
}
fn after_line(c: &Context<'_>, start: usize) -> usize {
    c.source.as_bytes()[start..]
        .iter()
        .position(|b| *b == b'\n')
        .map_or(c.source.as_bytes().len(), |i| start + i + 1)
}
fn after_terminator(c: &Context<'_>, opener: Node<'_>, mut offset: usize) -> Option<usize> {
    while offset < c.source.as_bytes().len() {
        let end = after_line(c, offset);
        let line = std::str::from_utf8(&c.source.as_bytes()[offset..end]).ok()?;
        if terminates(c, opener, line) {
            return Some(end);
        }
        offset = end;
    }
    None
}
fn terminates(c: &Context<'_>, opener: Node<'_>, line: &str) -> bool {
    let line = line.trim_end_matches(['\r', '\n']);
    let line = if c.text(opener).starts_with("<<~") || c.text(opener).starts_with("<<-") {
        line.trim_start_matches([' ', '\t'])
    } else {
        line
    };
    line == delimiter(c, opener)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;
    use crate::parse::source::SourceFile;
    #[test]
    fn nested_pending_heredocs_use_their_own_body_line() {
        let source = SourceFile::from_bytes("probe.rb", b"outer(<<~FIRST, <<~SECOND)\n #{class_eval(<<~CODE)}\n  1\n CODE\nFIRST\n second\nSECOND\n".to_vec());
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let config = CopConfig::default();
        let context = Context::new(&source, &config);
        let opener = nodes(tree.root_node(), "heredoc_beginning")
            .into_iter()
            .find(|n| context.text(*n) == "<<~CODE")
            .unwrap();
        assert_eq!(first_line(&context, opener), 3);
    }
}
