use crate::cop::shared::{named_kids, node_bytes};
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub fn word_value(source: &SourceFile, node: Node<'_>, array: Node<'_>) -> Option<String> {
    if named_kids(node).iter().any(|n| n.kind() == "interpolation") {
        return None;
    }
    let text = std::str::from_utf8(node_bytes(source, node)).ok()?;
    if !text.contains('\\') {
        return Some(text.into());
    }
    let syntax = node_bytes(source, array);
    let open = *syntax.get(2)?;
    let close = match open {
        b'[' => b']',
        b'(' => b')',
        b'{' => b'}',
        b'<' => b'>',
        other => other,
    };
    if matches!(syntax.get(1), Some(b'w' | b'i')) {
        let mut out = Vec::new();
        let mut bytes = text.bytes();
        while let Some(c) = bytes.next() {
            if c == b'\\' {
                let next = bytes.next()?;
                if !next.is_ascii_whitespace() && ![open, close, b'\\'].contains(&next) {
                    out.push(c);
                }
                out.push(next);
            } else {
                out.push(c);
            }
        }
        return String::from_utf8(out).ok();
    }
    let text = format!("%Q{}{text}{}", open as char, close as char);
    let parsed = SourceFile::from_bytes("word.rb", text.into_bytes());
    let tree = crate::parse::parse_ruby(&parsed).ok()?;
    crate::cop::ported::static_literal(&parsed, tree.root_node().named_child(0)?)
}
