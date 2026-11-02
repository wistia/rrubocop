//! Heuristic matchers for breadth-first Style cops.
use crate::cop::CopConfig;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub fn matches_block_delimiters(
    _source: &SourceFile,
    _node: Node<'_>,
    _config: &CopConfig,
) -> bool {
    false
}

pub fn matches_class_methods_definitions(
    _source: &SourceFile,
    _node: Node<'_>,
    _config: &CopConfig,
) -> bool {
    false
}

pub fn matches_conditional_assignment(
    _source: &SourceFile,
    _node: Node<'_>,
    _config: &CopConfig,
) -> bool {
    false
}

pub fn matches_if_inside_else(_source: &SourceFile, _node: Node<'_>, _config: &CopConfig) -> bool {
    false
}

pub fn matches_if_with_boolean_literal_branches(
    _source: &SourceFile,
    _node: Node<'_>,
    _config: &CopConfig,
) -> bool {
    false
}

pub fn matches_line_end_concatenation(
    _source: &SourceFile,
    _node: Node<'_>,
    _config: &CopConfig,
) -> bool {
    false
}

pub fn matches_method_call_with_args_parentheses(
    _source: &SourceFile,
    _node: Node<'_>,
    _config: &CopConfig,
) -> bool {
    false
}

pub fn matches_multiline_memoization(
    _source: &SourceFile,
    _node: Node<'_>,
    _config: &CopConfig,
) -> bool {
    false
}

pub fn matches_nested_parenthesized_calls(
    _source: &SourceFile,
    _node: Node<'_>,
    _config: &CopConfig,
) -> bool {
    false
}

pub fn matches_optional_arguments(
    _source: &SourceFile,
    _node: Node<'_>,
    _config: &CopConfig,
) -> bool {
    false
}

pub fn matches_redundant_line_continuation(
    _source: &SourceFile,
    _node: Node<'_>,
    _config: &CopConfig,
) -> bool {
    false
}

pub fn matches_redundant_string_escape(
    _source: &SourceFile,
    _node: Node<'_>,
    _config: &CopConfig,
) -> bool {
    false
}

pub fn matches_return_nil_in_predicate_method_definition(
    _source: &SourceFile,
    _node: Node<'_>,
    _config: &CopConfig,
) -> bool {
    false
}

pub fn matches_symbol_proc(_source: &SourceFile, _node: Node<'_>, _config: &CopConfig) -> bool {
    false
}
