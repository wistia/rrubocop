//! Ruby literal formatting for corrections requiring canonical dump/inspect spelling.
use crate::parse::source::SourceFile;
fn quote(value: &str, unicode: bool) -> String {
    let mut output = String::from("\"");
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\x07' => output.push_str("\\a"),
            '\x08' => output.push_str("\\b"),
            '\t' => output.push_str("\\t"),
            '\n' => output.push_str("\\n"),
            '\x0b' => output.push_str("\\v"),
            '\x0c' => output.push_str("\\f"),
            '\r' => output.push_str("\\r"),
            '\x1b' => output.push_str("\\e"),
            '#' if chars.peek().is_some_and(|c| matches!(c, '{' | '@' | '$')) => {
                output.push_str("\\#")
            }
            c if c.is_ascii_control() => output.push_str(&format!("\\x{:02X}", c as u32)),
            c if c.is_control() || unicode && !c.is_ascii() => {
                if c as u32 <= 0xffff {
                    output.push_str(&format!("\\u{:04X}", c as u32));
                } else {
                    output.push_str(&format!("\\u{{{:X}}}", c as u32));
                }
            }
            c => output.push(c),
        }
    }
    output.push('"');
    output
}
pub(crate) fn string_dump(value: &str) -> String {
    quote(value, true)
}
pub(crate) fn symbol_inspect(value: &str) -> String {
    let candidate = format!(":{value}");
    let source = SourceFile::from_bytes("symbol.rb", candidate.as_bytes().to_vec());
    if !matches!(value, "!@" | "~@")
        && crate::parse::parse_ruby(&source).ok().is_some_and(|tree| {
            let root = tree.root_node();
            !root.has_error()
                && root.named_child_count() == 1
                && root.named_child(0).is_some_and(|node| {
                    node.kind() == "simple_symbol" && node.end_byte() == candidate.len()
                })
        })
    {
        candidate
    } else {
        format!(":{}", quote(value, false))
    }
}
