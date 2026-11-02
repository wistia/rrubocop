//! Give enum members explicit integer values so inserting a member cannot shift existing values.
use crate::cop::shared::ruby_literal::{string_dump, symbol_inspect};
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, collect_comments, named_kids, node_bytes,
    node_text, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct EnumHash;
fn array(node: Node<'_>) -> bool {
    matches!(node.kind(), "array" | "string_array" | "symbol_array")
}
fn word_value(source: &SourceFile, node: Node<'_>, array: Node<'_>) -> Option<String> {
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
fn element(source: &SourceFile, node: Node<'_>, array: Node<'_>) -> Option<String> {
    if matches!(node.kind(), "bare_string" | "bare_symbol") {
        if named_kids(node).iter().any(|n| n.kind() == "interpolation") {
            return None;
        }
        let value = word_value(source, node, array)?;
        return Some(if array.kind() == "symbol_array" {
            symbol_inspect(&value)
        } else {
            string_dump(&value)
        });
    }
    if node.kind() == "character" {
        let text = node_text(source, node);
        let value = text.strip_prefix('?')?;
        if !value.starts_with('\\') {
            return Some(string_dump(value));
        }
        let parsed = SourceFile::from_bytes("character.rb", format!("\"{value}\"").into_bytes());
        let tree = crate::parse::parse_ruby(&parsed).ok()?;
        return crate::cop::ported::static_literal(&parsed, tree.root_node().named_child(0)?)
            .map(|s| string_dump(&s));
    }
    if let Some(value) = crate::cop::ported::static_literal(source, node) {
        return Some(
            if matches!(node.kind(), "simple_symbol" | "delimited_symbol") {
                symbol_inspect(&value)
            } else {
                string_dump(&value)
            },
        );
    }
    Some(node_text(source, node))
}
impl EnumHash {
    fn report(
        &self,
        source: &SourceFile,
        key: Node<'_>,
        array: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: &mut Option<&mut Vec<Correction>>,
    ) {
        let name = crate::cop::ported::static_literal(source, key)
            .unwrap_or_else(|| node_text(source, key));
        let (line, col) = source.offset_to_line_col(array.start_byte());
        let mut d=self.diagnostic(source,line,col,format!("Enum defined as an array found in `{name}` enum declaration. Use hash syntax instead."));
        d.highlight_length =
            super::blank::highlight_length(source, array.start_byte(), array.end_byte());
        let entries: Option<Vec<_>> = named_kids(array)
            .into_iter()
            .filter(|n| n.kind() != "comment")
            .enumerate()
            .map(|(i, n)| element(source, n, array).map(|s| format!("{s} => {i}")))
            .collect();
        d.correctable = !config.autocorrect_disabled()
            && entries.is_some()
            && std::str::from_utf8(node_bytes(source, array)).is_ok()
            && collect_comments(array).is_empty();
        if d.correctable {
            d.corrected = push_replace(
                corrections,
                array.start_byte(),
                array.end_byte(),
                format!("{{{}}}", entries.unwrap().join(", ")),
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}
impl Cop for EnumHash {
    fn name(&self) -> &'static str {
        "Rails/EnumHash"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn default_include(&self) -> &'static [&'static str] {
        &["**/app/models/**/*.rb"]
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"enum"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if call_method_name(source, node) != Some(b"enum") || call_receiver(node).is_some() {
            return;
        }
        let args = argument_nodes(node);
        if config.get_f64("TargetRailsVersion", 8.1) >= 7.0 && args.len() >= 2 && array(args[1]) {
            self.report(
                source,
                args[0],
                args[1],
                config,
                diagnostics,
                &mut corrections,
            );
        }
        let pairs = if let [options] = args.as_slice() {
            if options.kind() == "hash" {
                named_kids(*options)
            } else {
                args.clone()
            }
        } else if args
            .iter()
            .all(|n| matches!(n.kind(), "pair" | "hash_splat_argument"))
        {
            args
        } else {
            vec![]
        };
        for pair in pairs {
            if let (Some(key), Some(value)) = (
                pair.child_by_field_name("key"),
                pair.child_by_field_name("value"),
            ) {
                if array(value) {
                    self.report(source, key, value, config, diagnostics, &mut corrections);
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EnumHash, "cops/rails/enum_hash");
}
