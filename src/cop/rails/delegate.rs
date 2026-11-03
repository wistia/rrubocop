use crate::cop::shared::ruby_expression::{
    local_binding, plain_symbol, regular_send_arguments, regular_send_name, regular_send_receiver,
};
use crate::cop::shared::{for_each_descendant, named_kids, node_text, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct Delegate;
fn code_children(node: Node<'_>) -> Vec<Node<'_>> {
    named_kids(node)
        .into_iter()
        .filter(|n| n.kind() != "comment")
        .collect()
}
fn visibility(source: &SourceFile, node: Node<'_>) -> Option<String> {
    let name = regular_send_name(source, node)?;
    matches!(name.as_ref(), b"public" | b"private" | b"protected")
        .then(|| String::from_utf8_lossy(&name).into_owned())
        .filter(|_| regular_send_receiver(source, node) == Some(None))
}
fn public(source: &SourceFile, node: Node<'_>, method: &str) -> bool {
    let line = source.offset_to_line_col(node.start_byte()).0;
    let text = source
        .as_bytes()
        .split(|b| *b == b'\n')
        .nth(line - 1)
        .unwrap_or_default();
    let text = String::from_utf8_lossy(text);
    if text.trim_start().starts_with("private ") || text.contains("protected ") {
        return false;
    }
    let Some(mut parent) = node.parent() else {
        return true;
    };
    if parent.kind() == "argument_list" {
        if let Some(call) = parent.parent() {
            parent = call;
        }
    }
    if let Some(v) = visibility(source, parent) {
        if regular_send_arguments(source, parent).is_some_and(|a| a.len() == 1 && a[0] == node) {
            return v == "public";
        }
    }
    let siblings = code_children(parent);
    let Some(index) = siblings.iter().position(|n| *n == node) else {
        return true;
    };
    for sibling in siblings[index + 1..].iter().rev() {
        let Some(v) = visibility(source, *sibling) else {
            continue;
        };
        let args = regular_send_arguments(source, *sibling).unwrap_or_default();
        if args.len() == 1 && plain_symbol(source, args[0]).as_deref() == Some(method) {
            return v == "public";
        }
    }
    for sibling in siblings[..index].iter().rev() {
        let Some(v) = visibility(source, *sibling) else {
            continue;
        };
        if regular_send_arguments(source, *sibling).is_some_and(|a| a.is_empty()) {
            return v == "public";
        }
    }
    true
}
impl Cop for Delegate {
    fn name(&self) -> &'static str {
        "Rails/Delegate"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["method"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(name) = node.child_by_field_name("name") else {
            return;
        };
        let Some(body) = node.child_by_field_name("body") else {
            return;
        };
        let children = if body.kind() == "body_statement" {
            code_children(body)
        } else {
            vec![body]
        };
        if children.len() != 1 {
            return;
        }
        let body = children[0];
        if body.child_by_field_name("block").is_some() {
            return;
        }
        let Some(Some(receiver)) = regular_send_receiver(source, body) else {
            return;
        };
        let receiver_name = if receiver.kind() == "self" {
            "self".to_owned()
        } else {
            if regular_send_receiver(source, receiver) != Some(None)
                || !regular_send_arguments(source, receiver).is_some_and(|a| a.is_empty())
            {
                return;
            }
            let Some(n) = regular_send_name(source, receiver) else {
                return;
            };
            String::from_utf8_lossy(&n).into_owned()
        };
        let Some(target) = regular_send_name(source, body) else {
            return;
        };
        let target = String::from_utf8_lossy(&target);
        let method = node_text(source, name);
        let prefixed = receiver.kind() != "self" && method == format!("{receiver_name}_{target}");
        if method != target && !(prefixed && config.get_bool("EnforceForPrefixed", true)) {
            return;
        }
        let parameters = node
            .child_by_field_name("parameters")
            .map(code_children)
            .unwrap_or_default();
        let arguments = regular_send_arguments(source, body).unwrap_or_default();
        if parameters.len() != arguments.len()
            || !parameters.iter().zip(arguments).all(|(p, a)| {
                p.kind() == "identifier"
                    && a.kind() == "identifier"
                    && local_binding(source, a).is_some()
                    && node_text(source, *p) == node_text(source, a)
            })
        {
            return;
        }
        if !public(source, node, &method) {
            return;
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            "Use `delegate` to define delegations.".into(),
        );
        d.highlight_length = 3;
        if let Some(s) = config.severity {
            d.severity = s;
        }
        let mut comment = false;
        for_each_descendant(node, |n| comment |= n.kind() == "comment");
        d.correctable = !config.autocorrect_disabled() && !comment;
        if d.correctable {
            let receiver = if receiver.kind() == "self" {
                "self".into()
            } else {
                format!(":{receiver_name}")
            };
            let replacement = format!(
                "delegate :{target}, to: {receiver}{}",
                if prefixed { ", prefix: true" } else { "" }
            );
            d.corrected = push_replace(
                &mut corrections,
                node.start_byte(),
                node.end_byte(),
                replacement,
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}
