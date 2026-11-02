mod access_modifier_indentation;
mod align_items;
mod original_alignment;
mod argument_alignment;
mod array_alignment;
mod assignment_indentation;
mod begin_end_alignment;
mod block_alignment;
mod block_end_newline;
mod brace_layout;
mod case_indentation;
mod closing_heredoc_indentation;
mod closing_parenthesis_indentation;
mod comment_indentation;
mod condition_position;
mod def_end_alignment;
mod dot_position;
mod else_alignment;
mod empty_body;
mod empty_comment;
mod empty_line_after_guard_clause;
mod empty_line_after_magic_comment;
mod empty_line_between_defs;
mod empty_lines;
mod empty_lines_around_access_modifier;
mod empty_lines_around_arguments;
mod empty_lines_around_attribute_accessor;
mod empty_lines_around_begin_body;
mod empty_lines_around_block_body;
mod empty_lines_around_class_body;
mod empty_lines_around_exception_handling_keywords;
mod empty_lines_around_method_body;
mod empty_lines_around_module_body;
mod end_align;
mod end_alignment;
mod end_of_line;
mod extra_spacing;
mod first_argument_indentation;
mod first_array_element_indentation;
mod first_array_element_line_break;
mod first_break;
mod first_hash_element_indentation;
mod first_hash_element_line_break;
mod first_indent;
mod first_method_argument_line_break;
mod first_parameter_indentation;
mod hash_alignment;
mod heredoc_indentation;
mod indentation_consistency;
mod indentation_consistency_check;
mod indentation_consistency_util;
mod indentation_style;
mod indentation_width;
mod initial_indentation;
mod keyword_space;
mod leading_comment_space;
mod leading_empty_lines;
mod line_breaks;
mod line_continuation_leading_space;
mod line_continuation_spacing;
mod line_end_string_concatenation_indentation;
mod line_length;
mod multiline_array_brace_layout;
mod multiline_array_line_breaks;
mod multiline_block_layout;
mod multiline_hash_brace_layout;
mod multiline_hash_key_line_breaks;
mod multiline_method_argument_line_breaks;
mod multiline_method_call_brace_layout;
mod multiline_method_call_indentation;
mod multiline_method_definition_brace_layout;
mod multiline_operation_indentation;
mod parameter_alignment;
mod report;
mod rescue_ensure_alignment;
mod space_after_colon;
mod space_after_comma;
mod space_after_method_name;
mod space_after_not;
mod space_after_semicolon;
mod space_around_block_parameters;
mod space_around_equals_in_parameter_default;
mod space_around_keyword;
mod space_around_method_call_operator;
mod space_around_operators;
mod space_before_block_braces;
mod space_before_brackets;
mod space_before_comma;
mod space_before_comment;
mod space_before_first_arg;
mod space_before_semicolon;
mod space_delim;
mod space_in_lambda_literal;
mod space_inside_array_literal_brackets;
mod space_inside_array_percent_literal;
mod space_inside_block_braces;
mod space_inside_hash_literal_braces;
mod space_inside_parens;
mod space_inside_percent_literal_delimiters;
mod space_inside_range_literal;
mod space_inside_reference_brackets;
mod space_inside_string_interpolation;
mod trailing_empty_lines;
mod trailing_whitespace;

use crate::cop::registry::CopRegistry;

pub fn register_all(registry: &mut CopRegistry) {
    crate::register_cops!(registry;
        access_modifier_indentation::AccessModifierIndentation,
        argument_alignment::ArgumentAlignment,
        array_alignment::ArrayAlignment,
        assignment_indentation::AssignmentIndentation,
        begin_end_alignment::BeginEndAlignment,
        block_alignment::BlockAlignment,
        block_end_newline::BlockEndNewline,
        case_indentation::CaseIndentation,
        closing_heredoc_indentation::ClosingHeredocIndentation,
        closing_parenthesis_indentation::ClosingParenthesisIndentation,
        comment_indentation::CommentIndentation,
        condition_position::ConditionPosition,
        def_end_alignment::DefEndAlignment,
        dot_position::DotPosition,
        else_alignment::ElseAlignment,
        empty_comment::EmptyComment,
        empty_line_after_guard_clause::EmptyLineAfterGuardClause,
        empty_line_after_magic_comment::EmptyLineAfterMagicComment,
        empty_line_between_defs::EmptyLineBetweenDefs,
        empty_lines::EmptyLines,
        empty_lines_around_access_modifier::EmptyLinesAroundAccessModifier,
        empty_lines_around_arguments::EmptyLinesAroundArguments,
        empty_lines_around_attribute_accessor::EmptyLinesAroundAttributeAccessor,
        empty_lines_around_begin_body::EmptyLinesAroundBeginBody,
        empty_lines_around_block_body::EmptyLinesAroundBlockBody,
        empty_lines_around_class_body::EmptyLinesAroundClassBody,
        empty_lines_around_exception_handling_keywords::EmptyLinesAroundExceptionHandlingKeywords,
        empty_lines_around_method_body::EmptyLinesAroundMethodBody,
        empty_lines_around_module_body::EmptyLinesAroundModuleBody,
        end_alignment::EndAlignment,
        end_of_line::EndOfLine,
        extra_spacing::ExtraSpacing,
        first_argument_indentation::FirstArgumentIndentation,
        first_array_element_indentation::FirstArrayElementIndentation,
        first_array_element_line_break::FirstArrayElementLineBreak,
        first_hash_element_indentation::FirstHashElementIndentation,
        first_hash_element_line_break::FirstHashElementLineBreak,
        first_method_argument_line_break::FirstMethodArgumentLineBreak,
        first_parameter_indentation::FirstParameterIndentation,
        hash_alignment::HashAlignment,
        heredoc_indentation::HeredocIndentation,
        indentation_consistency::IndentationConsistency,
        indentation_style::IndentationStyle,
        indentation_width::IndentationWidth,
        initial_indentation::InitialIndentation,
        leading_comment_space::LeadingCommentSpace,
        leading_empty_lines::LeadingEmptyLines,
        line_length::LineLength,
        line_continuation_leading_space::LineContinuationLeadingSpace,
        line_continuation_spacing::LineContinuationSpacing,
        line_end_string_concatenation_indentation::LineEndStringConcatenationIndentation,
        multiline_array_brace_layout::MultilineArrayBraceLayout,
        multiline_array_line_breaks::MultilineArrayLineBreaks,
        multiline_block_layout::MultilineBlockLayout,
        multiline_hash_brace_layout::MultilineHashBraceLayout,
        multiline_hash_key_line_breaks::MultilineHashKeyLineBreaks,
        multiline_method_argument_line_breaks::MultilineMethodArgumentLineBreaks,
        multiline_method_call_brace_layout::MultilineMethodCallBraceLayout,
        multiline_method_call_indentation::MultilineMethodCallIndentation,
        multiline_method_definition_brace_layout::MultilineMethodDefinitionBraceLayout,
        multiline_operation_indentation::MultilineOperationIndentation,
        parameter_alignment::ParameterAlignment,
        rescue_ensure_alignment::RescueEnsureAlignment,
        space_after_colon::SpaceAfterColon,
        space_after_comma::SpaceAfterComma,
        space_after_method_name::SpaceAfterMethodName,
        space_after_not::SpaceAfterNot,
        space_after_semicolon::SpaceAfterSemicolon,
        space_around_block_parameters::SpaceAroundBlockParameters,
        space_around_equals_in_parameter_default::SpaceAroundEqualsInParameterDefault,
        space_around_keyword::SpaceAroundKeyword,
        space_around_method_call_operator::SpaceAroundMethodCallOperator,
        space_around_operators::SpaceAroundOperators,
        space_before_block_braces::SpaceBeforeBlockBraces,
        space_before_brackets::SpaceBeforeBrackets,
        space_before_comma::SpaceBeforeComma,
        space_before_comment::SpaceBeforeComment,
        space_before_first_arg::SpaceBeforeFirstArg,
        space_before_semicolon::SpaceBeforeSemicolon,
        space_in_lambda_literal::SpaceInLambdaLiteral,
        space_inside_array_literal_brackets::SpaceInsideArrayLiteralBrackets,
        space_inside_array_percent_literal::SpaceInsideArrayPercentLiteral,
        space_inside_block_braces::SpaceInsideBlockBraces,
        space_inside_hash_literal_braces::SpaceInsideHashLiteralBraces,
        space_inside_parens::SpaceInsideParens,
        space_inside_percent_literal_delimiters::SpaceInsidePercentLiteralDelimiters,
        space_inside_range_literal::SpaceInsideRangeLiteral,
        space_inside_reference_brackets::SpaceInsideReferenceBrackets,
        space_inside_string_interpolation::SpaceInsideStringInterpolation,
        trailing_empty_lines::TrailingEmptyLines,
        trailing_whitespace::TrailingWhitespace,
    );
}
