//! Rails/ReadWriteAttribute — breadth-first tree-sitter port.

use tree_sitter::Node;

use crate::cop::shared::{argument_nodes, call_method_name, node_text, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ReadWriteAttribute;

fn attribute_rewrite<'a>(source: &'a SourceFile, node: Node<'_>) -> Option<(String, &'a str)> {
    if node
        .child_by_field_name("receiver")
        .is_some_and(|n| n.kind() != "self")
    {
        return None;
    }
    let method = call_method_name(source, node)?;
    let args = argument_nodes(node);
    let repl = match method {
        b"read_attribute" if args.len() == 1 => {
            format!("self[{}]", node_text(source, args[0]))
        }
        b"write_attribute" if args.len() == 2 => {
            format!(
                "self[{}] = {}",
                node_text(source, args[0]),
                node_text(source, args[1])
            )
        }
        _ => return None,
    };
    Some((repl, std::str::from_utf8(method).unwrap_or("")))
}

impl Cop for ReadWriteAttribute {
    fn name(&self) -> &'static str {
        "Rails/ReadWriteAttribute"
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/app/models/**/*.rb"]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "command"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some((repl, method)) = attribute_rewrite(source, node) else {
            return;
        };
        if within_shadowing_method(source, node) {
            return;
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut diag = self.diagnostic(
            source,
            line,
            col,
            format!("Prefer `{repl}` over `{method}`."),
        );
        if push_replace(
            &mut corrections,
            node.start_byte(),
            node.end_byte(),
            repl,
            self.name(),
        ) {
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}

fn within_shadowing_method(source: &SourceFile, node: Node<'_>) -> bool {
    let Some(first) = argument_nodes(node).into_iter().next() else {
        return false;
    };
    let Some(attr) = sym_arg(source, first) else {
        return false;
    };
    let Some(method) = enclosing_method(node) else {
        return false;
    };
    let Some(name) = method
        .child_by_field_name("name")
        .map(|n| node_text(source, n))
    else {
        return false;
    };
    let shadow = if call_method_name(source, node) == Some(b"write_attribute") {
        format!("{attr}=")
    } else {
        attr
    };
    name == shadow
}

fn sym_arg(source: &SourceFile, node: Node<'_>) -> Option<String> {
    Some(node_text(source, node).trim_start_matches(':').to_string())
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
