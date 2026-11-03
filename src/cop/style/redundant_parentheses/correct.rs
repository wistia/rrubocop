use super::semantics::children;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub(super) struct Edit {
    pub start: usize,
    pub end: usize,
    pub text: &'static str,
}

pub(super) fn edits(
    source: &SourceFile,
    node: Node<'_>,
    start: usize,
    end: usize,
) -> Option<Vec<Edit>> {
    let bytes = source.as_bytes();
    let mut result = vec![Edit {
        start,
        end: start + 1,
        text: "",
    }];
    let mut close = end - 1;
    let mut close_end = end;
    let line_start = bytes[..close]
        .iter()
        .rposition(|b| *b == b'\n')
        .map_or(0, |i| i + 1);
    let mut comma = end;
    while bytes
        .get(comma)
        .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\r'))
    {
        comma += 1;
    }
    if bytes[line_start..close].iter().all(u8::is_ascii_whitespace)
        && bytes.get(comma) == Some(&b',')
    {
        while close > start + 1 && matches!(bytes[close - 1], b' ' | b'\t') {
            close -= 1;
        }
        while close >= start + 3 && &bytes[close - 2..close] == b"\\\n" {
            close -= 2;
        }
        while close > start + 1 && bytes[close - 1] == b'\n' {
            close -= 1;
        }
        while close > start + 1 && bytes[close - 1].is_ascii_whitespace() {
            close -= 1;
        }
        if crate::cop::shared::collect_comments(node)
            .iter()
            .any(|comment| comment.start_byte() < close && comment.end_byte() >= close)
        {
            return None;
        }
        if let Some(heredoc) = children(node)
            .last()
            .filter(|node| node.kind() == "heredoc_beginning")
        {
            close_end = comma + 1;
            result.push(Edit {
                start: heredoc.end_byte(),
                end: heredoc.end_byte(),
                text: ",",
            });
        }
    }
    result.push(Edit {
        start: close,
        end: close_end,
        text: "",
    });
    if bytes.get(end) == Some(&b'?')
        && node
            .parent()
            .is_some_and(|parent| parent.kind() == "conditional")
    {
        result.push(Edit {
            start: end,
            end,
            text: " ",
        });
    }
    Some(result)
}
