mod ambiguous_operator;
mod ambiguous_regexp_literal;
mod assignment_in_condition;
mod big_decimal_new;
mod binary_operator_with_identical_operands;
mod circular_argument_reference;
mod constant_definition_in_block;
mod constant_overwritten_in_rescue;
mod debugger;
mod deprecated_class_methods;
mod deprecated_open_ssl_constant;
mod duplicate_hash_key;
mod duplicate_magic_comment;
mod duplicate_match_pattern;
mod duplicate_methods;
mod each_with_object_argument;
mod else_layout;
mod empty_block;
mod empty_ensure;
mod empty_file;
mod empty_interpolation;
mod ensure_return;
mod flip_flop;
mod float_out_of_range;
mod format_parameter_mismatch;
mod implicit_string_concatenation;
mod ineffective_access_modifier;
mod inherit_exception;
mod it_without_arguments_in_block;
mod literal_as_condition;
mod literal_assignment_in_condition;
mod literal_in_interpolation;
mod loop_cop;
mod missing_cop_enable_directive;
mod missing_super;
mod mixed_case_range;
mod nested_method_definition;
mod next_without_accumulator;
mod no_return_in_begin_end_blocks;
mod non_deterministic_require_order;
mod non_local_exit_from_iterator;
mod ordered_magic_comments;
mod parentheses_as_grouped_expression;
mod percent_string_array;
mod percent_symbol_array;
mod rand_one;
mod redundant_cop_disable_directive;
mod redundant_cop_enable_directive;
mod redundant_regexp_quantifiers;
mod redundant_require_statement;
mod redundant_splat_expansion;
mod redundant_string_coercion;
mod require_parentheses;
mod require_range_parentheses;
mod require_relative_self_path;
mod rescue_exception;
mod safe_navigation_chain;
mod script_permission;
mod shadowed_exception;
mod suppressed_exception;
pub(crate) mod syntax;
pub use syntax::{Syntax, has_syntax_fatals};
mod underscore_prefixed_variable_name;
mod unified_integer;
mod unreachable_code;
mod unused_block_argument;
mod unused_method_argument;
mod useless_access_modifier;
mod useless_assignment;
mod useless_else_without_rescue;
mod useless_rescue;
mod useless_ruby2_keywords;
mod useless_setter_call;
mod void;

use crate::cop::registry::CopRegistry;

pub fn register_all(registry: &mut CopRegistry) {
    crate::register_cops!(registry;
        ambiguous_operator::AmbiguousOperator,
        ambiguous_regexp_literal::AmbiguousRegexpLiteral,
        assignment_in_condition::AssignmentInCondition,
        big_decimal_new::BigDecimalNew,
        binary_operator_with_identical_operands::BinaryOperatorWithIdenticalOperands,
        circular_argument_reference::CircularArgumentReference,
        constant_definition_in_block::ConstantDefinitionInBlock,
        constant_overwritten_in_rescue::ConstantOverwrittenInRescue,
        debugger::Debugger,
        deprecated_class_methods::DeprecatedClassMethods,
        deprecated_open_ssl_constant::DeprecatedOpenSSLConstant,
        duplicate_hash_key::DuplicateHashKey,
        duplicate_magic_comment::DuplicateMagicComment,
        duplicate_match_pattern::DuplicateMatchPattern,
        duplicate_methods::DuplicateMethods,
        each_with_object_argument::EachWithObjectArgument,
        else_layout::ElseLayout,
        empty_block::EmptyBlock,
        empty_ensure::EmptyEnsure,
        empty_file::EmptyFile,
        empty_interpolation::EmptyInterpolation,
        ensure_return::EnsureReturn,
        flip_flop::FlipFlop,
        float_out_of_range::FloatOutOfRange,
        format_parameter_mismatch::FormatParameterMismatch,
        implicit_string_concatenation::ImplicitStringConcatenation,
        ineffective_access_modifier::IneffectiveAccessModifier,
        inherit_exception::InheritException,
        it_without_arguments_in_block::ItWithoutArgumentsInBlock,
        literal_as_condition::LiteralAsCondition,
        literal_assignment_in_condition::LiteralAssignmentInCondition,
        literal_in_interpolation::LiteralInInterpolation,
        loop_cop::Loop,
        missing_cop_enable_directive::MissingCopEnableDirective,
        missing_super::MissingSuper,
        mixed_case_range::MixedCaseRange,
        nested_method_definition::NestedMethodDefinition,
        next_without_accumulator::NextWithoutAccumulator,
        no_return_in_begin_end_blocks::NoReturnInBeginEndBlocks,
        non_local_exit_from_iterator::NonLocalExitFromIterator,
        ordered_magic_comments::OrderedMagicComments,
        parentheses_as_grouped_expression::ParenthesesAsGroupedExpression,
        percent_string_array::PercentStringArray,
        percent_symbol_array::PercentSymbolArray,
        rand_one::RandOne,
        redundant_cop_disable_directive::RedundantCopDisableDirective,
        redundant_cop_enable_directive::RedundantCopEnableDirective,
        redundant_regexp_quantifiers::RedundantRegexpQuantifiers,
        redundant_require_statement::RedundantRequireStatement,
        redundant_splat_expansion::RedundantSplatExpansion,
        redundant_string_coercion::RedundantStringCoercion,
        require_parentheses::RequireParentheses,
        require_range_parentheses::RequireRangeParentheses,
        require_relative_self_path::RequireRelativeSelfPath,
        rescue_exception::RescueException,
        script_permission::ScriptPermission,
        safe_navigation_chain::SafeNavigationChain,
        shadowed_exception::ShadowedException,
        suppressed_exception::SuppressedException,
        syntax::Syntax,
        non_deterministic_require_order::NonDeterministicRequireOrder,
        underscore_prefixed_variable_name::UnderscorePrefixedVariableName,
        unified_integer::UnifiedInteger,
        unreachable_code::UnreachableCode,
        unused_block_argument::UnusedBlockArgument,
        unused_method_argument::UnusedMethodArgument,
        useless_access_modifier::UselessAccessModifier,
        useless_assignment::UselessAssignment,
        useless_else_without_rescue::UselessElseWithoutRescue,
        useless_rescue::UselessRescue,
        useless_ruby2_keywords::UselessRuby2Keywords,
        useless_setter_call::UselessSetterCall,
        void::Void,
    );
}
