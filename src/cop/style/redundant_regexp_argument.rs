//! Style/RedundantRegexpArgument: original deterministic source and quoting rules.
use crate::cop::shared::{call_method_name, node_bytes, push_replace, semantic_argument_nodes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct RedundantRegexpArgument;

fn preferred(content: &str) -> String {
    let mut text = String::new();
    let mut escaped = false;
    for ch in content.chars() {
        if !escaped && ch == '\\' {
            escaped = true;
            continue;
        }
        if escaped
            && matches!(
                ch,
                'a' | 'c'
                    | 'C'
                    | 'e'
                    | 'f'
                    | 'M'
                    | 'n'
                    | '"'
                    | '\''
                    | '\\'
                    | 't'
                    | 'b'
                    | 'r'
                    | 'u'
                    | 'v'
                    | 'x'
                    | '0'..='7'
            )
        {
            text.push('\\');
        }
        text.push(ch);
        escaped = false;
    }
    if text.contains('"') {
        format!("'{}'", text.replace('\'', "\\'"))
    } else {
        format!("\"{text}\"")
    }
}
fn replacement_value(prefer: &str) -> Option<String> {
    let source = SourceFile::from_bytes("replacement.rb", prefer.as_bytes().to_vec());
    let tree = crate::parse::parse_ruby(&source).ok()?;
    if tree.root_node().has_error() || tree.root_node().named_child_count() != 1 {
        return None;
    }
    let literal = tree.root_node().named_child(0)?;
    if literal.kind() != "string" {
        return None;
    }
    crate::cop::ported::static_ruby_string(&source, literal)
}
impl Cop for RedundantRegexpArgument {
    fn name(&self) -> &'static str {
        "Style/RedundantRegexpArgument"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if crate::cop::shared::is_setter_call(node) {
            return;
        }
        let Some(method) = call_method_name(source, node).filter(|m| {
            matches!(
                *m,
                b"byteindex"
                    | b"byterindex"
                    | b"gsub"
                    | b"gsub!"
                    | b"partition"
                    | b"rpartition"
                    | b"scan"
                    | b"split"
                    | b"start_with?"
                    | b"sub"
                    | b"sub!"
            )
        }) else {
            return;
        };
        let Some(regexp) = semantic_argument_nodes(node)
            .first()
            .copied()
            .filter(|n| n.kind() == "regex")
        else {
            return;
        };
        let Some(last) = regexp.child(regexp.child_count().saturating_sub(1) as u32) else {
            return;
        };
        if node_bytes(source, last).len() != 1 {
            return;
        }
        let Ok(raw) = std::str::from_utf8(node_bytes(source, regexp)) else {
            return;
        };
        if !crate::cop::shared::is_literal_regexp_text(raw) {
            return;
        }
        let Some(opening) = regexp.child(0) else {
            return;
        };
        let raw_content = &source.as_bytes()[opening.end_byte()..last.start_byte()];
        let Some(open) = node_bytes(source, opening).last().copied() else {
            return;
        };
        let Some(close) = node_bytes(source, last).first().copied() else {
            return;
        };
        let Some(content) = crate::cop::ported::decode_regexp_content(
            raw_content,
            open,
            close,
            config.get_f64("TargetRubyVersion", 2.7),
        )
        .and_then(|bytes| String::from_utf8(bytes).ok()) else {
            return;
        };
        let Some(expected) = crate::cop::shared::decode_ruby_double_quoted_content(&content) else {
            return;
        };
        if content == " " {
            return;
        }
        let prefer = preferred(&content);
        let (line, col) = source.offset_to_line_col(regexp.start_byte());
        let displayed = raw.replace("\r\n", "\n");
        let mut diagnostic = self.diagnostic(
            source,
            line,
            col,
            format!("Use string `{prefer}` as argument instead of regexp `{displayed}`."),
        );
        diagnostic.highlight_length = displayed.chars().count();
        let actual = replacement_value(&prefer);
        let safe = actual.as_deref() == Some(expected.as_str())
            && !(method == b"split" && expected == " ")
            && !crate::cop::shared::captures_observed(source, node);
        if safe {
            diagnostic.corrected = push_replace(
                &mut corrections,
                regexp.start_byte(),
                regexp.end_byte(),
                prefer,
                self.name(),
            );
        } else {
            diagnostic.correctable = false;
        }
        diagnostics.push(diagnostic);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_diagnostics_with_safeguards() {
        let mut cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/style/redundant_regexp_argument/intentional-safeguards.json"
        )))
        .unwrap();
        for case in cases.as_array_mut().unwrap() {
            case["ruby_corrected"] = case["rust_corrected"].clone();
        }
        crate::testutil::assert_original_spec_parity(&RedundantRegexpArgument, &cases.to_string());
    }
    #[test]
    fn original_ruby_specs() {
        crate::testutil::assert_original_spec_parity(
            &RedundantRegexpArgument,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/redundant_regexp_argument/parity.json"
            )),
        );
    }
}
