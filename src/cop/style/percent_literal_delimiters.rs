//! Style/PercentLiteralDelimiters — consistent `%`-literal delimiters.

use tree_sitter::Node;

use crate::cop::shared::node_bytes;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct PercentLiteralDelimiters;

fn percent_type(text: &[u8]) -> Option<&[u8]> {
    if !text.starts_with(b"%") || text.len() < 2 {
        return None;
    }
    if text[1].is_ascii_alphanumeric() {
        Some(&text[..2])
    } else {
        Some(&text[..1])
    }
}

fn open_delim(text: &[u8]) -> Option<u8> {
    if text.len() < 3 {
        return None;
    }
    let i = text
        .iter()
        .position(|&b| !b.is_ascii_alphanumeric() && b != b'%')?;
    text.get(i).copied()
}

fn close_for(open: u8) -> u8 {
    match open {
        b'(' => b')',
        b'[' => b']',
        b'{' => b'}',
        b'<' => b'>',
        c => c,
    }
}

fn rubocop_default_delimiters(ty: &[u8]) -> &'static str {
    match ty {
        b"%i" | b"%I" | b"%w" | b"%W" => "[]",
        b"%r" => "{}",
        _ => "()",
    }
}

fn mapping_pref<'a>(map: &'a serde_yml::Mapping, key: &str) -> Option<&'a str> {
    map.get(&serde_yml::Value::String(key.to_string()))
        .or_else(|| map.get(&serde_yml::Value::String("default".into())))
        .and_then(|v| v.as_str())
}

fn preferred_pair(config: &CopConfig, ty: &[u8]) -> (u8, u8) {
    let default = rubocop_default_delimiters(ty);
    let key = std::str::from_utf8(ty).unwrap_or("default");
    let mut chars = config
        .options
        .get("PreferredDelimiters")
        .and_then(|v| v.as_mapping())
        .and_then(|m| mapping_pref(m, key))
        .unwrap_or(default)
        .bytes();
    let open = chars.next().unwrap_or(b'(');
    (open, chars.next().unwrap_or(close_for(open)))
}

fn skip_delim_report(text: &[u8], ty: &[u8], used_open: u8, pref_open: u8, pref_close: u8) -> bool {
    used_open == pref_open
        || contains_delims(text, pref_open, pref_close)
        || ((ty == b"%w" || ty == b"%i") && contains_delims(text, used_open, close_for(used_open)))
}

fn contains_delims(text: &[u8], open: u8, close: u8) -> bool {
    percent_inner(text)
        .map(|s| s.contains(&open) || s.contains(&close))
        .unwrap_or(false)
}

fn percent_inner(text: &[u8]) -> Option<&[u8]> {
    let delim = open_delim(text)?;
    let oi = text.iter().position(|&b| b == delim)?;
    if oi >= text.len() {
        return None;
    }
    let close = close_for(text[oi]);
    let end = text.iter().rposition(|&b| b == close)?;
    if end <= oi {
        return None;
    }
    Some(&text[oi + 1..end])
}

fn percent_literal_kinds() -> &'static [&'static str] {
    &[
        "string_array",
        "symbol_array",
        "string",
        "%w",
        "%W",
        "%i",
        "%I",
        "%x",
        "regex",
    ]
}

impl Cop for PercentLiteralDelimiters {
    fn name(&self) -> &'static str {
        "Style/PercentLiteralDelimiters"
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        percent_literal_kinds()
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let text = node_bytes(source, node);
        let Some(ty) = percent_type(text) else {
            return;
        };
        let Some(used_open) = open_delim(text) else {
            return;
        };
        let (pref_open, pref_close) = preferred_pair(config, ty);
        if skip_delim_report(text, ty, used_open, pref_open, pref_close) {
            return;
        }
        let ty_s = std::str::from_utf8(ty).unwrap_or("%");
        let msg = format!(
            "`{ty_s}`-literals should be delimited by `{}` and `{}`.",
            pref_open as char, pref_close as char
        );
        let (line, col) = source.offset_to_line_col(node.start_byte());
        diagnostics.push(self.diagnostic(source, line, col, msg));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        PercentLiteralDelimiters,
        "cops/style/percent_literal_delimiters"
    );
}
