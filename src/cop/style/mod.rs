mod alias;
mod and_or;
mod arguments_forwarding;
mod array_intersect;
mod array_join;
mod attr;
mod bare_percent_literals;
mod begin_block;
mod block_comments;
mod block_delimiters;
mod case_equality;
mod case_like_if;
mod character_literal;
mod class_and_module_children;
mod class_check;
mod class_methods;
mod class_methods_definitions;
mod class_vars;
mod colon_method_call;
mod command_literal;
mod comment_annotation;
mod comparable_clamp;
mod concat_array_literals;
mod conditional_assignment;
mod data_inheritance;
mod date_time;
mod def_with_parentheses;
mod dir_empty;
mod each_for_simple_loop;
mod each_with_object;
mod empty_case_condition;
mod empty_else;
mod empty_heredoc;
mod empty_lambda_parameter;
mod empty_literal;
mod end_block;
mod endless_method;
mod even_odd;
mod exact_regexp_match;
mod explicit_block_argument;
mod file_empty;
mod for_cop;
mod format_string;
mod frozen_string_literal_comment;
mod global_std_stream;
mod global_vars;
mod hash_syntax;
pub(crate) mod heuristics;
mod identical_conditional_branches;
mod if_inside_else;
mod if_unless_modifier_of_if_unless;
mod if_with_boolean_literal_branches;
mod if_with_semicolon;
mod in_pattern_then;
mod infinite_loop;
mod invertible_unless_condition;
mod it_block_parameter;
mod lambda;
mod lambda_call;
mod line_end_concatenation;
mod magic_comment_format;
mod method_call_with_args_parentheses;
mod method_call_without_args_parentheses;
mod method_def_parentheses;
mod min_max_comparison;
mod missing_respond_to_missing;
mod module_function;
mod multiline_if_modifier;
mod multiline_if_then;
mod multiline_memoization;
mod multiline_ternary_operator;
mod negated_if;
mod negated_while;
mod nested_file_dirname;
mod nested_modifier;
mod nested_parenthesized_calls;
mod nested_ternary_operator;
mod next;
mod nil_comparison;
mod non_nil_check;
mod not;
mod numeric_literal_prefix;
mod one_line_conditional;
mod open_struct_use;
mod operator_method_call;
mod optional_arguments;
mod parallel_assignment;
mod parentheses_around_condition;
mod percent_literal_delimiters;
mod percent_q_literals;
mod perl_backrefs;
mod preferred_hash_methods;
mod proc;
mod quoted_symbols;
mod raise_args;
mod redundant_array_constructor;
mod redundant_begin;
mod redundant_capital_w;
mod redundant_constant_base;
mod redundant_current_directory_in_path;
mod redundant_double_splat_hash_braces;
mod redundant_each;
mod redundant_exception;
mod redundant_filter_chain;
mod redundant_freeze;
mod redundant_heredoc_delimiter_quotes;
mod redundant_interpolation;
mod redundant_line_continuation;
mod redundant_parentheses;
mod redundant_percent_q;
mod redundant_regexp_argument;
mod redundant_regexp_constructor;
mod redundant_return;
mod redundant_self;
mod redundant_sort_by;
mod redundant_string_escape;
mod regexp_literal;
mod rescue_modifier;
mod rescue_standard_error;
mod return_nil;
mod return_nil_in_predicate_method_definition;
mod safe_navigation;
mod sample;
mod self_assignment;
mod semicolon;
mod signal_exception;
mod single_line_do_end_block;
mod single_line_methods;
mod special_global_vars;
mod stabby_lambda_parentheses;
mod string_literals;
mod string_literals_in_interpolation;
mod strip;
mod super_with_args_parentheses;
mod symbol_array;
mod symbol_literal;
mod symbol_proc;
mod ternary_parentheses;
mod trailing_body_on_class;
mod trailing_body_on_module;
mod trailing_comma_args;
mod trailing_comma_in_arguments;
mod trailing_comma_in_array_literal;
pub(crate) mod trailing_comma_in_block_args;
mod trailing_comma_in_hash_literal;
mod trailing_comma_literal;
mod trivial_accessors;
mod unless_else;
mod variable_interpolation;
mod when_then;
mod while_until_do;
mod while_until_modifier;
mod word_array;
mod yaml_file_read;
mod zero_length_predicate;

use crate::cop::registry::CopRegistry;

pub fn register_all(registry: &mut CopRegistry) {
    crate::register_cops!(registry;
        alias::Alias,
        and_or::AndOr,
        arguments_forwarding::ArgumentsForwarding,
        array_intersect::ArrayIntersect,
        array_join::ArrayJoin,
        attr::Attr,
        bare_percent_literals::BarePercentLiterals,
        begin_block::BeginBlock,
        block_comments::BlockComments,
        block_delimiters::BlockDelimiters,
        case_equality::CaseEquality,
        case_like_if::CaseLikeIf,
        character_literal::CharacterLiteral,
        class_and_module_children::ClassAndModuleChildren,
        class_check::ClassCheck,
        class_methods::ClassMethods,
        class_methods_definitions::ClassMethodsDefinitions,
        class_vars::ClassVars,
        colon_method_call::ColonMethodCall,
        command_literal::CommandLiteral,
        comment_annotation::CommentAnnotation,
        comparable_clamp::ComparableClamp,
        concat_array_literals::ConcatArrayLiterals,
        conditional_assignment::ConditionalAssignment,
        data_inheritance::DataInheritance,
        date_time::DateTime,
        def_with_parentheses::DefWithParentheses,
        dir_empty::DirEmpty,
        each_for_simple_loop::EachForSimpleLoop,
        each_with_object::EachWithObject,
        empty_case_condition::EmptyCaseCondition,
        empty_else::EmptyElse,
        empty_heredoc::EmptyHeredoc,
        empty_lambda_parameter::EmptyLambdaParameter,
        empty_literal::EmptyLiteral,
        end_block::EndBlock,
        endless_method::EndlessMethod,
        even_odd::EvenOdd,
        exact_regexp_match::ExactRegexpMatch,
        explicit_block_argument::ExplicitBlockArgument,
        file_empty::FileEmpty,
        for_cop::For,
        format_string::FormatString,
        frozen_string_literal_comment::FrozenStringLiteralComment,
        global_std_stream::GlobalStdStream,
        global_vars::GlobalVars,
        hash_syntax::HashSyntax,
        identical_conditional_branches::IdenticalConditionalBranches,
        if_inside_else::IfInsideElse,
        if_unless_modifier_of_if_unless::IfUnlessModifierOfIfUnless,
        if_with_boolean_literal_branches::IfWithBooleanLiteralBranches,
        if_with_semicolon::IfWithSemicolon,
        in_pattern_then::InPatternThen,
        infinite_loop::InfiniteLoop,
        invertible_unless_condition::InvertibleUnlessCondition,
        it_block_parameter::ItBlockParameter,
        lambda::Lambda,
        lambda_call::LambdaCall,
        line_end_concatenation::LineEndConcatenation,
        magic_comment_format::MagicCommentFormat,
        method_call_with_args_parentheses::MethodCallWithArgsParentheses,
        method_call_without_args_parentheses::MethodCallWithoutArgsParentheses,
        method_def_parentheses::MethodDefParentheses,
        min_max_comparison::MinMaxComparison,
        missing_respond_to_missing::MissingRespondToMissing,
        module_function::ModuleFunction,
        multiline_if_modifier::MultilineIfModifier,
        multiline_if_then::MultilineIfThen,
        multiline_memoization::MultilineMemoization,
        multiline_ternary_operator::MultilineTernaryOperator,
        negated_if::NegatedIf,
        negated_while::NegatedWhile,
        nested_file_dirname::NestedFileDirname,
        nested_modifier::NestedModifier,
        nested_parenthesized_calls::NestedParenthesizedCalls,
        nested_ternary_operator::NestedTernaryOperator,
        next::Next,
        nil_comparison::NilComparison,
        non_nil_check::NonNilCheck,
        not::Not,
        numeric_literal_prefix::NumericLiteralPrefix,
        one_line_conditional::OneLineConditional,
        open_struct_use::OpenStructUse,
        operator_method_call::OperatorMethodCall,
        optional_arguments::OptionalArguments,
        parallel_assignment::ParallelAssignment,
        parentheses_around_condition::ParenthesesAroundCondition,
        percent_literal_delimiters::PercentLiteralDelimiters,
        percent_q_literals::PercentQLiterals,
        perl_backrefs::PerlBackrefs,
        preferred_hash_methods::PreferredHashMethods,
        proc::Proc,
        quoted_symbols::QuotedSymbols,
        raise_args::RaiseArgs,
        redundant_array_constructor::RedundantArrayConstructor,
        redundant_begin::RedundantBegin,
        redundant_capital_w::RedundantCapitalW,
        redundant_constant_base::RedundantConstantBase,
        redundant_current_directory_in_path::RedundantCurrentDirectoryInPath,
        redundant_double_splat_hash_braces::RedundantDoubleSplatHashBraces,
        redundant_each::RedundantEach,
        redundant_exception::RedundantException,
        redundant_filter_chain::RedundantFilterChain,
        redundant_freeze::RedundantFreeze,
        redundant_heredoc_delimiter_quotes::RedundantHeredocDelimiterQuotes,
        redundant_interpolation::RedundantInterpolation,
        redundant_line_continuation::RedundantLineContinuation,
        redundant_parentheses::RedundantParentheses,
        redundant_percent_q::RedundantPercentQ,
        redundant_regexp_argument::RedundantRegexpArgument,
        redundant_regexp_constructor::RedundantRegexpConstructor,
        redundant_return::RedundantReturn,
        redundant_self::RedundantSelf,
        redundant_sort_by::RedundantSortBy,
        redundant_string_escape::RedundantStringEscape,
        regexp_literal::RegexpLiteral,
        rescue_modifier::RescueModifier,
        rescue_standard_error::RescueStandardError,
        return_nil::ReturnNil,
        return_nil_in_predicate_method_definition::ReturnNilInPredicateMethodDefinition,
        safe_navigation::SafeNavigation,
        sample::Sample,
        self_assignment::SelfAssignment,
        semicolon::Semicolon,
        signal_exception::SignalException,
        single_line_do_end_block::SingleLineDoEndBlock,
        single_line_methods::SingleLineMethods,
        special_global_vars::SpecialGlobalVars,
        stabby_lambda_parentheses::StabbyLambdaParentheses,
        string_literals::StringLiterals,
        string_literals_in_interpolation::StringLiteralsInInterpolation,
        strip::Strip,
        super_with_args_parentheses::SuperWithArgsParentheses,
        symbol_array::SymbolArray,
        symbol_literal::SymbolLiteral,
        symbol_proc::SymbolProc,
        ternary_parentheses::TernaryParentheses,
        trailing_body_on_class::TrailingBodyOnClass,
        trailing_body_on_module::TrailingBodyOnModule,
        trailing_comma_in_arguments::TrailingCommaInArguments,
        trailing_comma_in_array_literal::TrailingCommaInArrayLiteral,
        trailing_comma_in_hash_literal::TrailingCommaInHashLiteral,
        trivial_accessors::TrivialAccessors,
        unless_else::UnlessElse,
        variable_interpolation::VariableInterpolation,
        when_then::WhenThen,
        while_until_do::WhileUntilDo,
        while_until_modifier::WhileUntilModifier,
        word_array::WordArray,
        yaml_file_read::YAMLFileRead,
        zero_length_predicate::ZeroLengthPredicate,
    );
}
