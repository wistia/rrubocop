//! Rename implicit Rails assertion aliases without changing custom/default-message behavior.

use tree_sitter::Node;

use crate::cop::shared::ruby_expression::{plain_string, regular_send_name, regular_send_receiver};
use crate::cop::shared::{argument_nodes, method_node, named_kids, node_bytes, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RefuteMethods;

const CORRECTIONS: &[(&[u8], &str)] = &[
    (b"refute", "assert_not"),
    (b"refute_empty", "assert_not_empty"),
    (b"refute_equal", "assert_not_equal"),
    (b"refute_in_delta", "assert_not_in_delta"),
    (b"refute_in_epsilon", "assert_not_in_epsilon"),
    (b"refute_includes", "assert_not_includes"),
    (b"refute_instance_of", "assert_not_instance_of"),
    (b"refute_kind_of", "assert_not_kind_of"),
    (b"refute_nil", "assert_not_nil"),
    (b"refute_operator", "assert_not_operator"),
    (b"refute_predicate", "assert_not_predicate"),
    (b"refute_respond_to", "assert_not_respond_to"),
    (b"refute_same", "assert_not_same"),
    (b"refute_match", "assert_no_match"),
];

fn style_pair<'a>(style: &str, method: &'a [u8]) -> Option<(&'a str, &'a str)> {
    let method_str = std::str::from_utf8(method).unwrap_or("");
    if style == "assert_not" {
        let (_, good) = CORRECTIONS.iter().find(|(b, _)| *b == method)?;
        Some((method_str, *good))
    } else {
        let (bad, _) = CORRECTIONS.iter().find(|(_, g)| g.as_bytes() == method)?;
        Some((method_str, std::str::from_utf8(bad).unwrap_or("")))
    }
}

struct AssertionOverrides(bool);
fn overridden(source: &SourceFile, node: Node<'_>) -> bool {
    source
        .cached(0, || {
            let mut root = node;
            while let Some(parent) = root.parent() {
                root = parent;
            }
            let mut pending = vec![root];
            while let Some(node) = pending.pop() {
                if matches!(node.kind(), "method" | "singleton_method")
                    && node.child_by_field_name("name").is_some_and(|name| {
                        CORRECTIONS.iter().any(|(bad, good)| {
                            node_bytes(source, name) == *bad
                                || node_bytes(source, name) == good.as_bytes()
                        })
                    })
                {
                    return AssertionOverrides(true);
                }
                if node.kind() == "alias"
                    || regular_send_name(source, node).is_some_and(|name| {
                        matches!(
                            name.as_ref(),
                            b"alias_method"
                                | b"define_method"
                                | b"define_singleton_method"
                                | b"class_eval"
                                | b"module_eval"
                                | b"eval"
                                | b"prepend"
                        )
                    })
                {
                    return AssertionOverrides(true);
                }
                pending.extend(named_kids(node));
            }
            AssertionOverrides(false)
        })
        .0
}
impl Cop for RefuteMethods {
    fn name(&self) -> &'static str {
        "Rails/RefuteMethods"
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/test/**/*"]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "identifier"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(method) = regular_send_name(source, node) else {
            return;
        };
        if regular_send_receiver(source, node) != Some(None) {
            return;
        }
        let Some((bad, good)) = style_pair(config.get_str("EnforcedStyle", "assert_not"), &method)
        else {
            return;
        };
        let meth = method_node(node).unwrap_or(node);
        let (line, col) = source.offset_to_line_col(meth.start_byte());
        let mut diag = self.diagnostic(source, line, col, format!("Prefer `{good}` over `{bad}`."));
        diag.highlight_length = meth.end_byte() - meth.start_byte();
        let args = argument_nodes(node);
        let plain = matches!(method.as_ref(), b"refute" | b"assert_not");
        let same_message = !plain
            || args
                .first()
                .is_some_and(|arg| matches!(arg.kind(), "nil" | "false"))
            || args
                .get(1)
                .is_some_and(|arg| plain_string(source, *arg).is_some());
        diag.correctable = !config.autocorrect_disabled()
            && node.kind() != "identifier"
            && same_message
            && !overridden(source, node);
        if diag.correctable
            && push_replace(
                &mut corrections,
                meth.start_byte(),
                meth.end_byte(),
                good,
                self.name(),
            )
        {
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}
