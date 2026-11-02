//! Lint/NonDeterministicRequireOrder, gated on sorted glob defaults in Ruby 3.0.
use crate::cop::shared::{
    call_method_name, call_receiver, method_node, node_bytes, push_replace, semantic_argument_nodes,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct NonDeterministicRequireOrder;

fn ordinary(source: &SourceFile, node: Node<'_>) -> bool {
    node.kind() == "call"
        && !node
            .child_by_field_name("operator")
            .is_some_and(|op| node_bytes(source, op) == b"&.")
}
fn dir_glob(source: &SourceFile, node: Node<'_>) -> bool {
    ordinary(source, node)
        && call_method_name(source, node) == Some(b"glob")
        && call_receiver(node)
            .is_some_and(|recv| matches!(node_bytes(source, recv), b"Dir" | b"::Dir"))
}
fn dir_index(source: &SourceFile, node: Node<'_>) -> bool {
    (node.kind() == "element_reference"
        && node
            .child_by_field_name("object")
            .is_some_and(|recv| matches!(node_bytes(source, recv), b"Dir" | b"::Dir")))
        || (ordinary(source, node)
            && call_method_name(source, node) == Some(b"[]")
            && call_receiver(node)
                .is_some_and(|recv| matches!(node_bytes(source, recv), b"Dir" | b"::Dir")))
}
fn dir_each(source: &SourceFile, node: Node<'_>) -> bool {
    ordinary(source, node)
        && call_method_name(source, node) == Some(b"each")
        && call_receiver(node).is_some_and(|recv| dir_glob(source, recv) || dir_index(source, recv))
}
fn required_variable(source: &SourceFile, body: Node<'_>, variable: &[u8]) -> bool {
    let mut found = false;
    crate::cop::shared::for_each_descendant(body, |node| {
        if !ordinary(source, node)
            || call_receiver(node).is_some()
            || !matches!(
                call_method_name(source, node),
                Some(b"require" | b"require_relative")
            )
        {
            return;
        }
        let args = semantic_argument_nodes(node);
        found |= args.len() == 1
            && args[0].kind() == "identifier"
            && node_bytes(source, args[0]) == variable;
    });
    found
}
fn outer_numbered_parameter(source: &SourceFile, node: Node<'_>, variable: &[u8]) -> bool {
    if matches!(
        node.kind(),
        "block"
            | "do_block"
            | "lambda"
            | "method"
            | "singleton_method"
            | "class"
            | "module"
            | "singleton_class"
    ) {
        return false;
    }
    if node.kind() == "identifier"
        && node_bytes(source, node) == variable
        && node.parent().is_none_or(|parent| {
            parent.child_by_field_name("method") != Some(node)
                && parent.child_by_field_name("name") != Some(node)
        })
    {
        return true;
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| outer_numbered_parameter(source, child, variable))
}
fn require_pass(source: &SourceFile, argument: Node<'_>) -> bool {
    if argument.kind() != "block_argument" {
        return false;
    }
    let Some(method) = argument.named_child(0) else {
        return false;
    };
    if !ordinary(source, method)
        || call_receiver(method).is_some()
        || call_method_name(source, method) != Some(b"method")
    {
        return false;
    }
    let args = semantic_argument_nodes(method);
    args.len() == 1
        && matches!(
            node_bytes(source, args[0]),
            b":require" | b":require_relative"
        )
}
fn call_end(source: &SourceFile, node: Node<'_>) -> usize {
    let mut end = node
        .child_by_field_name("block")
        .map_or(node.end_byte(), |block| block.start_byte());
    while end > node.start_byte() && source.as_bytes()[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    end
}
impl Cop for NonDeterministicRequireOrder {
    fn name(&self) -> &'static str {
        "Lint/NonDeterministicRequireOrder"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn safe_autocorrect(&self) -> bool {
        false
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
        if config.get_f64("TargetRubyVersion", 2.7) >= 3.0
            || !(dir_glob(source, node) || dir_each(source, node))
        {
            return;
        }
        let args = semantic_argument_nodes(node);
        let pass = args
            .last()
            .copied()
            .filter(|arg| require_pass(source, *arg));
        let mut requires_sort = false;
        if let Some(block) = node.child_by_field_name("block") {
            if !args.is_empty() && dir_each(source, node) {
                return;
            }
            let Some(body) = block.child_by_field_name("body") else {
                return;
            };
            if let Some(parameters) = block.child_by_field_name("parameters") {
                let mut cursor = parameters.walk();
                let params: Vec<_> = parameters
                    .named_children(&mut cursor)
                    .filter(|child| child.kind() != "comment")
                    .collect();
                if params.len() == 1
                    && params[0].kind() == "identifier"
                    && required_variable(source, body, node_bytes(source, params[0]))
                {
                    requires_sort = true;
                }
            } else {
                for number in b'1'..=b'9' {
                    if outer_numbered_parameter(source, body, &[b'_', number])
                        && required_variable(source, body, &[b'_', number])
                    {
                        requires_sort = true;
                        break;
                    }
                }
            }
        } else if pass.is_some() && (dir_glob(source, node) || args.len() == 1) {
            requires_sort = true;
        }
        if !requires_sort {
            return;
        }
        let end = call_end(source, node);
        let (line, column) = source.offset_to_line_col(node.start_byte());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Sort files before requiring them.".into(),
        );
        diagnostic.highlight_length =
            String::from_utf8_lossy(&source.as_bytes()[node.start_byte()..end])
                .chars()
                .count();
        if dir_each(source, node) {
            let method = method_node(node).unwrap();
            diagnostic.corrected = push_replace(
                &mut corrections,
                method.start_byte(),
                method.end_byte(),
                "sort.each",
                self.name(),
            );
        } else if let Some(pass) = pass {
            let Some(previous) = args
                .get(args.len().saturating_sub(2))
                .filter(|arg| arg.id() != pass.id())
            else {
                diagnostic.correctable = false;
                diagnostics.push(diagnostic);
                return;
            };
            let removes_comment =
                crate::cop::shared::collect_comments(node)
                    .iter()
                    .any(|comment| {
                        comment.start_byte() >= previous.end_byte()
                            && comment.end_byte() <= pass.start_byte()
                    });
            if removes_comment {
                diagnostic.correctable = false;
            } else {
                diagnostic.corrected = push_replace(
                    &mut corrections,
                    previous.end_byte(),
                    pass.end_byte(),
                    "",
                    self.name(),
                );
                push_replace(
                    &mut corrections,
                    end,
                    end,
                    format!(
                        ".sort.each({})",
                        String::from_utf8_lossy(node_bytes(source, pass))
                    ),
                    self.name(),
                );
            }
        } else {
            diagnostic.corrected =
                push_replace(&mut corrections, end, end, ".sort.each", self.name());
        }
        diagnostics.push(diagnostic);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn does_not_delete_comments_while_moving_block_pass() {
        let diagnostics = crate::testutil::run_cop_full(
            &NonDeterministicRequireOrder,
            b"Dir.glob('*.rb', # retain\n &method(:require))\n",
        );
        assert_eq!(diagnostics.len(), 1);
        assert!(!diagnostics[0].correctable);
    }
    #[test]
    fn missing_glob_arguments_do_not_crash_autocorrection() {
        let diagnostics = crate::testutil::run_cop_full(
            &NonDeterministicRequireOrder,
            b"Dir.glob(&method(:require))\n",
        );
        assert_eq!(diagnostics.len(), 1);
        assert!(!diagnostics[0].correctable);
    }
    #[test]
    fn original_ruby_specs() {
        crate::testutil::assert_original_spec_parity(
            &NonDeterministicRequireOrder,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/lint/non_deterministic_require_order/parity.json"
            )),
        );
    }
}
