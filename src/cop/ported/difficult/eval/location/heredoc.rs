use super::*;
pub(super) fn first_line(c: &Context<'_>, code: Node<'_>) -> usize {
    if code.kind() != "heredoc_beginning" {
        return code.start_position().row + 1;
    }
    let root = ancestors(code).last().unwrap_or(code);
    let index = crate::cop::shared::heredoc_index::heredoc_index(c.source, root);
    index.unique_body(delimiter(c, code)).map_or_else(
        || lexical_line(c, code, root),
        |body| body_line(c, body.node.start),
    )
}

fn delimiter<'a>(c: &Context<'a>, code: Node<'_>) -> &'a str {
    c.text(code)
        .trim_start_matches("<<")
        .trim_start_matches(['~', '-'])
        .trim_matches(['\'', '"', '`'])
}
fn body_line(c: &Context<'_>, start: usize) -> usize {
    let tail = &c.source.as_bytes()[start..];
    let newline = if tail.starts_with(b"\r\n") {
        2
    } else {
        usize::from(tail.starts_with(b"\n"))
    };
    c.source.offset_to_line_col(start + newline).0
}

fn lexical_line(c: &Context<'_>, code: Node<'_>, root: Node<'_>) -> usize {
    c.source.offset_to_line_col(lexical_offset(c, code, root)).0
}
fn lexical_offset(c: &Context<'_>, code: Node<'_>, root: Node<'_>) -> usize {
    let start = after_line(c, code.end_byte());
    let index = crate::cop::shared::heredoc_index::heredoc_index(c.source, root);
    index
        .preceding_on_line(code.start_byte(), code.start_position().row)
        .try_fold(start, |offset, opener| {
            after_terminator(c, &c.source.as_bytes()[opener.start..opener.end], offset)
        })
        .unwrap_or(start)
}
fn after_line(c: &Context<'_>, start: usize) -> usize {
    c.source.as_bytes()[start..]
        .iter()
        .position(|b| *b == b'\n')
        .map_or(c.source.as_bytes().len(), |i| start + i + 1)
}
fn after_terminator(c: &Context<'_>, opener: &[u8], mut offset: usize) -> Option<usize> {
    while offset < c.source.as_bytes().len() {
        let end = after_line(c, offset);
        let line = std::str::from_utf8(&c.source.as_bytes()[offset..end]).ok()?;
        if terminates(opener, line) {
            return Some(end);
        }
        offset = end;
    }
    None
}
fn terminates(opener: &[u8], line: &str) -> bool {
    let line = line.trim_end_matches(['\r', '\n']);
    let line = if opener.starts_with(b"<<~") || opener.starts_with(b"<<-") {
        line.trim_start_matches([' ', '\t'])
    } else {
        line
    };
    line == std::str::from_utf8(opener)
        .unwrap_or("")
        .trim_start_matches("<<")
        .trim_start_matches(['~', '-'])
        .trim_matches(['\'', '"', '`'])
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
        let opener = descendants(tree.root_node())
            .filter(|node| node.kind() == "heredoc_beginning")
            .find(|n| context.text(*n) == "<<~CODE")
            .unwrap();
        assert_eq!(first_line(&context, opener), 3);
    }
}
