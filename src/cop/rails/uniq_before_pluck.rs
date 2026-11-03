//! Move uniqueness into a relation without discarding Ruby argument/block evaluation.
use crate::cop::shared::ruby_expression::{regular_send_name, regular_send_receiver};
use crate::cop::shared::{collect_comments, named_kids, node_bytes, node_text, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct UniqBeforePluck;
fn ordinary_block(source: &SourceFile, node: Node<'_>, config: &CopConfig) -> bool {
    node.child_by_field_name("block").is_some_and(|block| {
        config.get_f64("TargetRubyVersion", 3.4) < 2.7
            || crate::cop::ported::numbered_bindings(source, block).is_none()
    })
}
fn custom_relation(source: &SourceFile, node: Node<'_>) -> bool {
    struct CustomRelation(bool);
    source
        .cached(0, || CustomRelation(custom_relation_uncached(source, node)))
        .0
}
fn custom_relation_uncached(source: &SourceFile, node: Node<'_>) -> bool {
    let mut root = node;
    while let Some(parent) = root.parent() {
        root = parent;
    }
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if matches!(node.kind(), "method" | "singleton_method")
            && node
                .child_by_field_name("name")
                .is_some_and(|n| matches!(node_bytes(source, n), b"pluck" | b"distinct" | b"uniq"))
        {
            return true;
        }
        if regular_send_name(source, node).is_some_and(|name| {
            matches!(
                name.as_ref(),
                b"define_method"
                    | b"define_singleton_method"
                    | b"class_eval"
                    | b"module_eval"
                    | b"eval"
                    | b"prepend"
            )
        }) {
            return true;
        }
        stack.extend(named_kids(node));
    }
    false
}
impl Cop for UniqBeforePluck {
    fn name(&self) -> &'static str {
        "Rails/UniqBeforePluck"
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"uniq"]
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn safe_autocorrect(&self) -> bool {
        false
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if regular_send_name(source, node).as_deref() != Some(b"uniq")
            || ordinary_block(source, node, config)
        {
            return;
        }
        let Some(Some(pluck)) = regular_send_receiver(source, node) else {
            return;
        };
        if pluck.child_by_field_name("block").is_some()
            || regular_send_name(source, pluck).as_deref() != Some(b"pluck")
        {
            return;
        }
        let Some(receiver) = regular_send_receiver(source, pluck) else {
            return;
        };
        if config.get_str("EnforcedStyle", "conservative") == "conservative"
            && !receiver.is_some_and(|n| {
                matches!(n.kind(), "constant" | "scope_resolution")
                    || (n.kind() == "identifier" && node_bytes(source, n) == b"__ENCODING__")
            })
        {
            return;
        }
        let Some(selector) = node.child_by_field_name("method") else {
            return;
        };
        let (line, col) = source.offset_to_line_col(selector.start_byte());
        let mut d = self.diagnostic(source, line, col, "Use `distinct` before `pluck`.".into());
        d.highlight_length = 4;
        d.correctable = false;
        if !config.autocorrect_disabled()
            && node.child_by_field_name("arguments").is_none()
            && node.child_by_field_name("block").is_none()
            && !custom_relation(source, node)
        {
            if let Some(dot) = node.child_by_field_name("operator") {
                let text = node_text(source, node);
                let last = text.rsplit('\n').next().unwrap_or("");
                let start = if last.trim_matches(|c: char| c.is_ascii_whitespace()) == ".uniq" {
                    node.start_byte() + text.rfind('\n').unwrap_or(0)
                } else {
                    dot.start_byte()
                };
                let insert = pluck
                    .child_by_field_name("operator")
                    .map_or(pluck.start_byte(), |n| n.start_byte());
                let replacement = if receiver.is_some() {
                    ".distinct"
                } else {
                    "distinct."
                };
                let comments = collect_comments(node);
                let removes_comment = comments
                    .iter()
                    .any(|n| n.start_byte() < selector.end_byte() && n.end_byte() > start);
                let root_text = String::from_utf8_lossy(source.as_bytes());
                let line_sensitive = root_text.contains("__LINE__")
                    && source.as_bytes()[start..selector.end_byte()].contains(&b'\n');
                if !removes_comment
                    && !line_sensitive
                    && !source.as_bytes().windows(2).any(|b| b == b"\r\n")
                {
                    let mut proposed = source.as_bytes().to_vec();
                    proposed.splice(start..selector.end_byte(), []);
                    proposed.splice(insert..insert, replacement.bytes());
                    let proposed = SourceFile::from_bytes("pluck.rb", proposed);
                    d.correctable = crate::parse::parse_ruby(&proposed)
                        .is_ok_and(|t| !t.root_node().has_error());
                    if d.correctable {
                        let first = push_replace(
                            &mut corrections,
                            start,
                            selector.end_byte(),
                            "",
                            self.name(),
                        );
                        let second = push_replace(
                            &mut corrections,
                            insert,
                            insert,
                            replacement,
                            self.name(),
                        );
                        d.corrected = first && second;
                    }
                }
            }
        }
        diagnostics.push(d);
    }
}
