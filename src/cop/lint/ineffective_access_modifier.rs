use tree_sitter::Node;

use crate::cop::shared::node_bytes;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Lint/IneffectiveAccessModifier — private/protected before defs of self.
pub struct IneffectiveAccessModifier;

fn access_name(source: &SourceFile, node: Node<'_>) -> Option<&'static str> {
    if node.kind() != "identifier" {
        return None;
    }
    match node_bytes(source, node) {
        b"private" => Some("private"),
        b"protected" => Some("protected"),
        b"public" => Some("public"),
        _ => None,
    }
}

fn report_singleton(
    cop: &IneffectiveAccessModifier,
    source: &SourceFile,
    child: Node<'_>,
    line: usize,
    modifier: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if modifier == "public" {
        return;
    }
    let (l, col) = source.offset_to_line_col(child.start_byte());
    diagnostics.push(cop.diagnostic(
        source,
        l,
        col,
        format!(
            "`{modifier}` (on line {line}) does not make singleton methods private. Use `private_class_method` or `private` inside a `class << self` block instead."
        ),
    ));
}

fn explicitly_private(source: &SourceFile, body: Node<'_>, child: Node<'_>) -> bool {
    let Some(name) = child.child_by_field_name("name") else {
        return false;
    };
    let name = node_bytes(source, name);
    let mut private_cursor = body.walk();
    body.named_children(&mut private_cursor).any(|n| {
        if crate::cop::shared::call_method_name(source, n) != Some(b"private_class_method") {
            return false;
        }
        crate::cop::shared::argument_nodes(n)
            .iter()
            .any(|arg| node_bytes(source, *arg).strip_prefix(b":") == Some(name))
    })
}

fn scan_body(
    cop: &IneffectiveAccessModifier,
    source: &SourceFile,
    body: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut cur = body.walk();
    let mut current_mod: Option<(usize, &str)> = None;
    for child in body.named_children(&mut cur) {
        if let Some(modifier) = access_name(source, child) {
            let (line, _) = source.offset_to_line_col(child.start_byte());
            current_mod = Some((line, modifier));
            continue;
        }
        if child.kind() != "singleton_method" {
            continue;
        }
        if explicitly_private(source, body, child) {
            continue;
        }
        if let Some((line, modifier)) = current_mod {
            report_singleton(cop, source, child, line, modifier, diagnostics);
        }
    }
}

impl Cop for IneffectiveAccessModifier {
    fn name(&self) -> &'static str {
        "Lint/IneffectiveAccessModifier"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["class", "module"]
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
        scan_body(self, source, body, diagnostics);
    }
}
