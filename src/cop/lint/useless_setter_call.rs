use tree_sitter::Node;

use crate::cop::shared::{for_each_descendant, node_bytes, node_text};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Lint/UselessSetterCall — setter on local that is discarded as method value.
pub struct UselessSetterCall;

fn last_assignment(body: Node<'_>) -> Option<Node<'_>> {
    let mut cur = body.walk();
    let last = body.named_children(&mut cur).last()?;
    (last.kind() == "assignment").then_some(last)
}

fn setter_call<'a>(node: Node<'a>) -> Option<(Node<'a>, Node<'a>)> {
    let left = node.child_by_field_name("left")?;
    if left.kind() != "call" {
        return None;
    }
    let recv = left.child_by_field_name("receiver")?;
    (recv.kind() == "identifier").then_some((left, recv))
}

fn new_call(source: &SourceFile, node: Node<'_>) -> bool {
    node.kind() == "call" && crate::cop::shared::call_method_name(source, node) == Some(b"new")
}

fn created_from_call(source: &SourceFile, body: Node<'_>, var: &[u8], skip: usize) -> bool {
    let mut found = false;
    for_each_descendant(body, |n| {
        if n.kind() != "assignment" || n.id() == skip {
            return;
        }
        let Some(l) = n.child_by_field_name("left") else {
            return;
        };
        if l.kind() != "identifier" || node_bytes(source, l) != var {
            return;
        }
        if n.child_by_field_name("right")
            .is_some_and(|r| new_call(source, r))
        {
            found = true;
        }
    });
    found
}

impl Cop for UselessSetterCall {
    fn name(&self) -> &'static str {
        "Lint/UselessSetterCall"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["method", "singleton_method"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let Some(body) = node.child_by_field_name("body") else {
            return;
        };
        let Some(last) = last_assignment(body) else {
            return;
        };
        let Some((left, recv)) = setter_call(last) else {
            return;
        };
        if !created_from_call(source, body, node_bytes(source, recv), last.id()) {
            return;
        }
        let v = node_text(source, recv);
        let (line, col) = source.offset_to_line_col(left.start_byte());
        diagnostics.push(self.diagnostic(
            source,
            line,
            col,
            format!("Useless setter call to local variable `{v}`."),
        ));
    }
}
