//! Style/SafeNavigation — prefer &. over explicit nil checks (breadth).

mod context;
use context::unsafe_enclosing_use;

use tree_sitter::Node;

use crate::cop::shared::{call_method_name, call_receiver, node_bytes, node_text};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SafeNavigation;

impl Cop for SafeNavigation {
    fn name(&self) -> &'static str {
        "Style/SafeNavigation"
    }

    /// Still misses chained `x && x.y.z` and `x.y if x.y` — skip redundant-disable audit.
    fn redundant_disable_audit(&self) -> bool {
        false
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["binary", "unless_modifier", "if_modifier"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if let Some(diag) = modifier_offense(self, source, node) {
            diagnostics.push(diag);
            return;
        }
        if node.kind() != "binary"
            || !is_and_safe_nav(source, node)
            || lhs_reassigned_in_method(source, node)
            || inside_block(node)
            || unsafe_enclosing_use(source, node)
        {
            return;
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        diagnostics.push(self.diagnostic(
            source,
            line,
            col,
            "Use safe navigation (`&.`) instead of checking `nil` with `&&`.".to_string(),
        ));
    }
}

fn modifier_offense<'a>(
    cop: &SafeNavigation,
    source: &'a SourceFile,
    node: Node<'a>,
) -> Option<Diagnostic> {
    match node.kind() {
        "unless_modifier" => unless_nil_safe_nav(cop, source, node),
        "if_modifier" => if_modifier_safe_nav(cop, source, node),
        _ => None,
    }
}

fn unless_nil_safe_nav(
    cop: &SafeNavigation,
    source: &SourceFile,
    node: Node<'_>,
) -> Option<Diagnostic> {
    let cond = node.child_by_field_name("condition")?;
    if !is_nil_check(source, cond) {
        return None;
    }
    let body = node.child_by_field_name("body")?;
    let recv = nil_check_receiver(source, cond)?;
    if !call_on_receiver(source, body, recv) {
        return None;
    }
    modifier_safe_nav_diag(cop, source, body)
}

/// `foo.bar if foo` → `foo&.bar` (RuboCop Style/SafeNavigation).
fn if_modifier_safe_nav(
    cop: &SafeNavigation,
    source: &SourceFile,
    node: Node<'_>,
) -> Option<Diagnostic> {
    let cond = node.child_by_field_name("condition")?;
    if cond.kind() != "identifier" {
        return None;
    }
    let body = node.child_by_field_name("body")?;
    if !call_on_receiver(source, body, cond) {
        return None;
    }
    modifier_safe_nav_diag(cop, source, body)
}

fn modifier_safe_nav_diag(
    cop: &SafeNavigation,
    source: &SourceFile,
    body: Node<'_>,
) -> Option<Diagnostic> {
    let (line, col) = source.offset_to_line_col(body.start_byte());
    Some(cop.diagnostic(
        source,
        line,
        col,
        "Use safe navigation (`&.`) instead of checking if an object exists before calling the method.".to_string(),
    ))
}

fn is_nil_check(source: &SourceFile, node: Node<'_>) -> bool {
    node.kind() == "call" && call_method_name(source, node) == Some(b"nil?")
}

fn nil_check_receiver<'a>(_source: &'a SourceFile, node: Node<'a>) -> Option<Node<'a>> {
    call_receiver(node).filter(|r| r.kind() == "identifier" || r.kind() == "call")
}

fn call_on_receiver(source: &SourceFile, node: Node<'_>, recv: Node<'_>) -> bool {
    if node.kind() != "call" {
        return false;
    }
    call_receiver(node).is_some_and(|r| node_bytes(source, r) == node_bytes(source, recv))
}

fn is_and_safe_nav(source: &SourceFile, node: Node<'_>) -> bool {
    let mut cur = node.walk();
    let kids: Vec<_> = node.children(&mut cur).collect();
    if kids.len() < 3 {
        return false;
    }
    if node_bytes(source, kids[1]) != b"&&" {
        return false;
    }
    let right = kids[2];
    if right.kind() != "call" {
        return false;
    }
    let Some(recv) = right.child_by_field_name("receiver") else {
        return false;
    };
    node_bytes(source, kids[0]) == node_bytes(source, recv)
}

fn inside_block(node: Node<'_>) -> bool {
    let mut p = node.parent();
    while let Some(n) = p {
        if matches!(n.kind(), "block" | "do_block" | "lambda") {
            return true;
        }
        p = n.parent();
    }
    false
}

fn lhs_reassigned_in_method(source: &SourceFile, node: Node<'_>) -> bool {
    let mut cur = node.walk();
    let Some(lhs) = node.children(&mut cur).collect::<Vec<_>>().first().copied() else {
        return false;
    };
    let Some(name) = local_name(source, lhs) else {
        return false;
    };
    let Some(method) = enclosing_method(node) else {
        return false;
    };
    method_reassigns_name(source, method, &name, lhs.start_byte())
}

fn local_name(source: &SourceFile, node: Node<'_>) -> Option<String> {
    (node.kind() == "identifier").then(|| node_text(source, node))
}

fn enclosing_method(node: Node<'_>) -> Option<Node<'_>> {
    let mut p = node.parent();
    while let Some(n) = p {
        if matches!(n.kind(), "method" | "singleton_method") {
            return Some(n);
        }
        p = n.parent();
    }
    None
}

fn method_reassigns_name(source: &SourceFile, method: Node<'_>, name: &str, before: usize) -> bool {
    let Some(body) = method.child_by_field_name("body") else {
        return false;
    };
    let mut out = false;
    walk_assignments(source, body, name, before, &mut out);
    out
}

fn walk_assignments(
    source: &SourceFile,
    node: Node<'_>,
    name: &str,
    before: usize,
    found: &mut bool,
) {
    if *found {
        return;
    }
    if node.kind() == "assignment" {
        if let Some(left) = node.child_by_field_name("left") {
            if left.kind() == "identifier"
                && node_text(source, left) == name
                && node.start_byte() < before
            {
                *found = true;
                return;
            }
        }
    }
    let mut cur = node.walk();
    for child in node.children(&mut cur) {
        walk_assignments(source, child, name, before, found);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SafeNavigation, "cops/style/safe_navigation");
}
