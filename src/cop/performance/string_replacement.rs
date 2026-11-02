use crate::cop::shared::ruby_expression::{plain_string, static_regex_value_for_ruby};
use crate::cop::shared::ruby_literal::string_literal;
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, captures_observed, collect_comments,
    decode_ruby_double_quoted_content, is_literal_regexp_text, method_node, node_bytes, node_text,
    push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
fn interpret(text: &str) -> Option<String> {
    let mut normalized = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        normalized.push(c);
        if c == '\\' {
            if let Some(next) = chars.next() {
                if matches!(next, 'c' | 'C' | 'M') {
                    normalized.pop();
                }
                normalized.push(next);
            }
        }
    }
    decode_ruby_double_quoted_content(&normalized)
}
fn pattern(source: &SourceFile, node: Node<'_>, config: &CopConfig) -> Option<(String, bool)> {
    if let Some(text) = plain_string(source, node) {
        return Some((text, true));
    }
    let text = if node.kind() == "regex" {
        static_regex_value_for_ruby(source, node, config.get_f64("TargetRubyVersion", 3.4))?
    } else {
        if !matches!(call_method_name(source, node), Some(b"new" | b"compile"))
            || !call_receiver(node)
                .is_some_and(|r| r.kind() == "constant" && node_bytes(source, r) == b"Regexp")
            || node.child_by_field_name("block").is_some()
            || node
                .child_by_field_name("operator")
                .is_some_and(|op| node_bytes(source, op) == b"&.")
        {
            return None;
        }
        let args = argument_nodes(node);
        if args.len() != 1 {
            return None;
        }
        if args[0].kind() == "regex" {
            static_regex_value_for_ruby(source, args[0], config.get_f64("TargetRubyVersion", 3.4))?
        } else {
            plain_string(source, args[0])?
        }
    };
    if !is_literal_regexp_text(&text) {
        return None;
    }
    Some((interpret(&text)?, false))
}
fn visible_override(source: &SourceFile, mut node: Node<'_>) -> bool {
    while let Some(parent) = node.parent() {
        node = parent;
    }
    struct VisibleOverrides(bool);
    source
        .cached(0, || {
            let mut found = false;
            crate::cop::shared::for_each_descendant(node, |n| {
                found |= matches!(n.kind(), "method" | "singleton_method")
                    && n.child_by_field_name("name").is_some_and(|name| {
                        matches!(
                            node_bytes(source, name),
                            b"new"
                                | b"compile"
                                | b"gsub"
                                | b"gsub!"
                                | b"tr"
                                | b"tr!"
                                | b"delete"
                                | b"delete!"
                        )
                    });
                found |= matches!(
                    call_method_name(source, n),
                    Some(b"define_method" | b"define_singleton_method" | b"alias_method")
                ) && argument_nodes(n)
                    .first()
                    .and_then(|name| crate::cop::ported::static_symbol(source, *name))
                    .is_some_and(|name| {
                        matches!(
                            name.as_str(),
                            "new"
                                | "compile"
                                | "gsub"
                                | "gsub!"
                                | "tr"
                                | "tr!"
                                | "delete"
                                | "delete!"
                        )
                    });
            });
            VisibleOverrides(found)
        })
        .0
}
pub struct StringReplacement;
impl Cop for StringReplacement {
    fn name(&self) -> &'static str {
        "Performance/StringReplacement"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"gsub", b"gsub!"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(method @ (b"gsub" | b"gsub!")) = call_method_name(source, node) else {
            return;
        };
        let args = argument_nodes(node);
        if args.len() != 2 {
            return;
        }
        let (Some((first, string)), Some(second)) = (
            pattern(source, args[0], config),
            plain_string(source, args[1]),
        ) else {
            return;
        };
        if first.chars().count() != 1 || second.chars().count() > 1 {
            return;
        }
        let delete = second.is_empty();
        let replacement = format!(
            "{}{}",
            if delete { "delete" } else { "tr" },
            if method == b"gsub!" { "!" } else { "" }
        );
        let Some(selector) = method_node(node) else {
            return;
        };
        let end = super::send_range::end(node);
        let (line, col) = source.offset_to_line_col(selector.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            format!(
                "Use `{replacement}` instead of `{}`.",
                String::from_utf8_lossy(method)
            ),
        );
        d.highlight_length =
            String::from_utf8_lossy(&source.as_bytes()[selector.start_byte()..end])
                .split('\n')
                .next()
                .unwrap_or("")
                .chars()
                .count();
        d.correctable = !config.autocorrect_disabled();
        let known = call_receiver(node).is_some_and(|r| plain_string(source, r).is_some());
        if d.correctable
            && known
            && !captures_observed(source, node)
            && !visible_override(source, node)
            && collect_comments(node).is_empty()
            && !node_text(source, args[0]).contains("<<")
        {
            let mut edits = vec![(selector.start_byte(), selector.end_byte(), replacement)];
            if !string {
                edits.push((
                    args[0].start_byte(),
                    args[0].end_byte(),
                    string_literal(&first),
                ));
            }
            if delete {
                let suffix = if node
                    .child_by_field_name("arguments")
                    .is_some_and(|a| node_bytes(source, a).starts_with(b"("))
                {
                    ")"
                } else {
                    ""
                };
                edits.push((args[0].end_byte(), end, suffix.into()));
            }
            let mut candidate = source.as_bytes().to_vec();
            let mut ordered = edits.clone();
            ordered.sort_by_key(|e| std::cmp::Reverse(e.0));
            for (start, end, text) in ordered {
                candidate.splice(start..end, text.bytes());
            }
            let candidate = SourceFile::from_bytes("candidate.rb", candidate);
            if crate::parse::parse_ruby(&candidate).is_ok_and(|tree| !tree.root_node().has_error())
            {
                for (start, end, text) in edits {
                    d.corrected |= push_replace(&mut corrections, start, end, text, self.name());
                }
            }
        }
        diagnostics.push(d);
    }
}
