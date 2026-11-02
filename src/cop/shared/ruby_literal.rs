//! Ruby literal formatting for corrections requiring canonical dump/inspect spelling.
use crate::parse::source::SourceFile;
#[derive(Clone, Copy, PartialEq, Eq)]
enum QuoteStyle {
    Dump,
    Inspect,
    Symbol,
}
fn quote(value: &str, style: QuoteStyle) -> String {
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
            c if c.is_ascii_control() && style != QuoteStyle::Inspect => {
                output.push_str(&format!("\\x{:02X}", c as u32))
            }
            c if c.is_control()
                || matches!(c, '\u{2028}' | '\u{2029}')
                || style == QuoteStyle::Dump && !c.is_ascii() =>
            {
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
pub(crate) fn string_literal(value: &str) -> String {
    let inspected = quote(value, QuoteStyle::Inspect);
    let mut chars = inspected[1..inspected.len() - 1].chars().peekable();
    let mut double = false;
    while let Some(c) = chars.next() {
        if c == '\'' {
            double = true;
            break;
        }
        if c == '\\' && chars.next().is_some_and(|next| !matches!(next, '\\' | '"')) {
            double = true;
            break;
        }
    }
    if double {
        inspected
    } else {
        format!("'{}'", value.replace('\\', "\\\\"))
    }
}
pub(crate) fn string_dump(value: &str) -> String {
    quote(value, QuoteStyle::Dump)
}
pub(crate) fn symbol_inspect(value: &str) -> String {
    let candidate = format!(":{value}");
    let source = SourceFile::from_bytes("symbol.rb", candidate.as_bytes().to_vec());
    if !matches!(value, "!@" | "~@")
        && !value
            .chars()
            .any(|c| c.is_ascii_control() || matches!(c, '\u{2028}' | '\u{2029}'))
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
        format!(":{}", quote(value, QuoteStyle::Symbol))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn util_string_literal_matches_original_inspect_and_quote_policy() {
        for (value, expected) in [
            ("text", "'text'"),
            ("é", "'é'"),
            ("a'b", "\"a'b\""),
            ("a\u{1}", "\"a\\u0001\""),
            ("a\u{7f}", "\"a\\u007F\""),
            ("\u{2028}", "\"\\u2028\""),
            ("#{x}", "\"\\#{x}\""),
            ("\\", "'\\\\'"),
            ("a\nb", "\"a\\nb\""),
        ] {
            assert_eq!(string_literal(value), expected);
        }
    }

    #[test]
    fn dump_symbol_and_string_inspect_keep_distinct_control_spelling() {
        assert_eq!(string_dump("a\u{1}"), "\"a\\x01\"");
        assert_eq!(symbol_inspect("a\u{1}"), ":\"a\\x01\"");
        assert_eq!(symbol_inspect("\u{2028}"), ":\"\\u2028\"");
    }
}
