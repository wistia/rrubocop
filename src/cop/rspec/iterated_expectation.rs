use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::shared::{call_receiver, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct IteratedExpectation;
fn expectation(
    source: &SourceFile,
    node: Node<'_>,
    argument: &[u8],
    numbered: bool,
    header: bool,
) -> bool {
    if !runner(source, node)
        || name(source, node) != Some(b"to")
        || !header && call_block(node).is_some()
    {
        return false;
    }
    let Some(expect) = call_receiver(node) else {
        return false;
    };
    if !bare(source, expect, b"expect") || safe_send(source, expect) || call_block(expect).is_some()
    {
        return false;
    }
    let args = call_args(expect);
    args.len() == 1
        && args[0].kind() == "identifier"
        && node_bytes(source, args[0]) == argument
        && (local_binding(source, args[0]) || numbered && argument == b"_1")
}
fn direct_children(node: Node<'_>) -> Vec<Node<'_>> {
    if node.kind() == "call" {
        let mut children: Vec<_> = call_receiver(node).into_iter().collect();
        children.extend(call_args(node));
        return children;
    }
    let mut children = Vec::new();
    for child in code_children(node) {
        if node.kind() == "assignment" && node.child_by_field_name("left") == Some(child) {
            if child.kind() == "scope_resolution" {
                children.extend(child.child_by_field_name("scope"));
            }
            continue;
        }
        if child.kind() == "argument_list" {
            children.extend(code_children(child));
        } else if matches!(
            child.kind(),
            "then" | "else" | "do" | "body_statement" | "block_body"
        ) && code_children(child).len() == 1
        {
            children.extend(code_children(child));
        } else {
            children.push(child);
        }
    }
    children
}
impl Cop for IteratedExpectation {
    fn name(&self) -> &'static str {
        "RSpec/IteratedExpectation"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if safe_send(source, node)
            || name(source, node) != Some(b"each")
            || !call_args(node).is_empty()
        {
            return;
        }
        let Some(block) = call_block(node) else {
            return;
        };
        let numbered = numbered_block(source, node);
        let argument = if numbered {
            b"_1".as_slice()
        } else {
            let Some(parameters) = block.child_by_field_name("parameters") else {
                return;
            };
            let parameters = code_children(parameters);
            if parameters.len() != 1 || parameters[0].kind() != "identifier" {
                return;
            }
            node_bytes(source, parameters[0])
        };
        let Some(body) = block.child_by_field_name("body") else {
            return;
        };
        let statements = code_children(body);
        let offensive = if statements.len() == 1 {
            let body = statements[0];
            if expectation(source, body, argument, numbered, false) {
                true
            } else if call_block(body).is_some() {
                numbered_block(source, body)
                    && expectation(source, body, argument, numbered, true)
                    && call_block(body)
                        .and_then(|block| block.child_by_field_name("body"))
                        .is_some_and(|body| {
                            code_children(body).len() == 1
                                && expectation(
                                    source,
                                    code_children(body)[0],
                                    argument,
                                    numbered,
                                    false,
                                )
                        })
            } else {
                let children = direct_children(body);
                !children.is_empty()
                    && children
                        .into_iter()
                        .all(|child| expectation(source, child, argument, numbered, false))
            }
        } else {
            !statements.is_empty()
                && statements
                    .into_iter()
                    .all(|statement| expectation(source, statement, argument, numbered, false))
        };
        if offensive {
            report(
                self,
                source,
                node.start_byte(),
                header_end(node),
                "Prefer using the `all` matcher instead of iterating over an array.".into(),
                diagnostics,
            );
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(IteratedExpectation, "cops/rspec/iterated_expectation");
    fn assert_report_only(bytes: &[u8]) {
        let source = SourceFile::from_bytes("t_spec.rb", bytes.to_vec());
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let mut diagnostics = Vec::new();
        let mut corrections = Vec::new();
        let config = CopConfig::default();
        crate::cop::walker::BatchedWalker::new(vec![&IteratedExpectation], vec![&config]).walk(
            &source,
            tree.root_node(),
            &mut diagnostics,
            Some(&mut corrections),
        );
        assert_eq!(diagnostics.len(), 1);
        assert!(!diagnostics[0].correctable);
        assert!(corrections.is_empty());
    }
    #[test]
    fn keeps_the_original_ruby_report_only_contract() {
        assert_report_only(b"[a, b].each { |value| expect(value).to be_valid }\n");
    }
    #[test]
    fn keeps_element_dependent_matchers_unchanged() {
        assert_report_only(b"[a].each { |value| expect(value).to eq(value) }\n");
    }
    #[test]
    fn keeps_local_matcher_names_unchanged() {
        assert_report_only(b"[a].each { |be_valid| expect(be_valid).to be_valid }\n");
    }
    #[test]
    fn keeps_multiple_expectations_unchanged() {
        assert_report_only(
            b"[a].each { |value| expect(value).to be_valid; expect(value).to be_ok }\n",
        );
    }
}
