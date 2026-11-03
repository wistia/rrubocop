//! Heuristic matchers for breadth-first Style cops.

mod calls;
pub use calls::matches_class_and_module_children;

mod collections;
pub use collections::matches_empty_heredoc;
pub use collections::matches_symbol_array;
pub use collections::matches_word_array;
pub(crate) use collections::word_symbol_name;

mod control;
pub use control::matches_if_with_semicolon;
pub use control::matches_one_line_conditional;
pub use control::matches_stabby_lambda_parentheses;

mod invertible_unless_condition;
pub use invertible_unless_condition::matches_invertible_unless_condition;

mod percent;
pub use percent::matches_bare_percent_literals;
pub use percent::matches_command_literal;
pub use percent::matches_percent_q_literals;

mod string_literals;
pub(crate) use string_literals::double_quotes_required;
pub use string_literals::matches_string_literals;

mod redundant_misc;
pub use redundant_misc::matches_redundant_capital_w;
pub use redundant_misc::matches_redundant_double_splat_hash_braces;
pub use redundant_misc::matches_redundant_heredoc_delimiter_quotes;
pub use redundant_misc::matches_redundant_interpolation;
pub use redundant_misc::matches_redundant_percent_q;

mod identical_branches;
pub use identical_branches::identical_branch_nodes;
#[allow(unused_imports)]
pub use identical_branches::matches_identical_conditional_branches;

mod explicit_block_argument;
pub use explicit_block_argument::matches_explicit_block_argument;

mod stubs;
pub use stubs::matches_block_delimiters;
pub use stubs::matches_class_methods_definitions;
pub use stubs::matches_conditional_assignment;
pub use stubs::matches_if_inside_else;
pub use stubs::matches_if_with_boolean_literal_branches;
pub use stubs::matches_line_end_concatenation;
pub use stubs::matches_method_call_with_args_parentheses;
pub use stubs::matches_multiline_memoization;
pub use stubs::matches_nested_parenthesized_calls;
pub use stubs::matches_optional_arguments;
pub use stubs::matches_redundant_line_continuation;
pub use stubs::matches_redundant_string_escape;
pub use stubs::matches_return_nil_in_predicate_method_definition;
pub use stubs::matches_symbol_proc;
