mod any_instance;
mod around_block;
mod be;
mod be_empty;
mod be_eq;
mod be_eql;
mod be_nil;
mod before_after_all;
mod change_by_zero;
mod class_check;
mod contain_exactly;
mod context_method;
mod context_wording;
mod describe_class;
mod describe_method;
mod describe_symbol;
mod described_class;
mod duplicated_metadata;
mod empty_example_group;
mod empty_hook;
mod empty_line_after_example;
mod empty_line_after_example_group;
mod empty_line_after_final_let;
mod empty_line_after_hook;
mod empty_line_after_subject;
mod empty_metadata;
mod empty_output;
mod eq;
mod example_length;
mod example_without_description;
mod example_wording;
mod excessive_docstring_spacing;
mod expect_actual;
mod expect_change;
mod expect_in_hook;
mod expect_in_let;
mod expect_output;
mod focus;
pub(crate) mod helpers;
mod hook_argument;
mod hooks_before_examples;
mod identical_equality_assertion;
mod implicit_block_expectation;
mod implicit_expect;
mod implicit_subject;
mod indexed_let;
mod instance_spy;
mod instance_variable;
mod is_expected_specify;
mod it_behaves_like;
mod iterated_expectation;
mod leading_subject;
mod leaky_constant_declaration;
mod let_before_examples;
mod let_setup;
mod match_array;
mod message_chain;
mod message_spies;
mod metadata_style;
mod missing_example_group_argument;
mod missing_expectation_target_method;
mod multiple_describes;
mod multiple_expectations;
mod multiple_memoized_helpers;
mod multiple_subjects;
mod named_subject;
mod nested_groups;
mod no_expectation_example;
mod not_to_not;
mod overwriting_setup;
mod parity_helpers;
mod pending_without_reason;
mod predicate_matcher;
mod receive_counts;
mod receive_messages;
mod receive_never;
mod redundant_around;
mod redundant_predicate_matcher;
mod remove_const;
mod repeated_description;
mod repeated_example;
mod repeated_example_group_body;
mod repeated_example_group_description;
mod repeated_include_example;
mod repeated_subject_call;
mod return_from_stub;
mod scattered_let;
mod scattered_setup;
mod separation;
mod shared_context;
mod shared_examples;
mod single_argument_message_chain;
mod skip_block_inside_example;
mod sort_metadata;
mod spec_file_path_format;
mod spec_file_path_suffix;
mod stubbed_mock;
mod subject_declaration;
mod subject_stub;
mod undescriptive_literals_description;
mod unspecified_exception;
mod variable_definition;
mod variable_name;
mod verified_double_reference;
mod verified_doubles;
mod void_expect;
mod yield_cop;

use crate::cop::registry::CopRegistry;

pub fn register_all(registry: &mut CopRegistry) {
    crate::register_cops!(registry;
        any_instance::AnyInstance,
        around_block::AroundBlock,
        be::Be,
        be_empty::BeEmpty,
        be_eq::BeEq,
        be_eql::BeEql,
        be_nil::BeNil,
        before_after_all::BeforeAfterAll,
        change_by_zero::ChangeByZero,
        class_check::ClassCheck,
        contain_exactly::ContainExactly,
        context_method::ContextMethod,
        context_wording::ContextWording,
        describe_class::DescribeClass,
        describe_method::DescribeMethod,
        describe_symbol::DescribeSymbol,
        described_class::DescribedClass,
        duplicated_metadata::DuplicatedMetadata,
        empty_example_group::EmptyExampleGroup,
        empty_hook::EmptyHook,
        empty_line_after_example::EmptyLineAfterExample,
        empty_line_after_example_group::EmptyLineAfterExampleGroup,
        empty_line_after_final_let::EmptyLineAfterFinalLet,
        empty_line_after_hook::EmptyLineAfterHook,
        empty_line_after_subject::EmptyLineAfterSubject,
        empty_metadata::EmptyMetadata,
        empty_output::EmptyOutput,
        eq::Eq,
        example_length::ExampleLength,
        example_without_description::ExampleWithoutDescription,
        example_wording::ExampleWording,
        excessive_docstring_spacing::ExcessiveDocstringSpacing,
        expect_actual::ExpectActual,
        expect_change::ExpectChange,
        expect_in_hook::ExpectInHook,
        expect_in_let::ExpectInLet,
        expect_output::ExpectOutput,
        focus::Focus,
        hook_argument::HookArgument,
        hooks_before_examples::HooksBeforeExamples,
        identical_equality_assertion::IdenticalEqualityAssertion,
        implicit_block_expectation::ImplicitBlockExpectation,
        implicit_expect::ImplicitExpect,
        implicit_subject::ImplicitSubject,
        indexed_let::IndexedLet,
        instance_spy::InstanceSpy,
        instance_variable::InstanceVariable,
        is_expected_specify::IsExpectedSpecify,
        it_behaves_like::ItBehavesLike,
        iterated_expectation::IteratedExpectation,
        leading_subject::LeadingSubject,
        leaky_constant_declaration::LeakyConstantDeclaration,
        let_before_examples::LetBeforeExamples,
        let_setup::LetSetup,
        match_array::MatchArray,
        message_chain::MessageChain,
        message_spies::MessageSpies,
        metadata_style::MetadataStyle,
        missing_example_group_argument::MissingExampleGroupArgument,
        missing_expectation_target_method::MissingExpectationTargetMethod,
        multiple_describes::MultipleDescribes,
        multiple_expectations::MultipleExpectations,
        multiple_memoized_helpers::MultipleMemoizedHelpers,
        multiple_subjects::MultipleSubjects,
        named_subject::NamedSubject,
        nested_groups::NestedGroups,
        no_expectation_example::NoExpectationExample,
        not_to_not::NotToNot,
        overwriting_setup::OverwritingSetup,
        pending_without_reason::PendingWithoutReason,
        predicate_matcher::PredicateMatcher,
        receive_counts::ReceiveCounts,
        receive_messages::ReceiveMessages,
        receive_never::ReceiveNever,
        redundant_around::RedundantAround,
        redundant_predicate_matcher::RedundantPredicateMatcher,
        remove_const::RemoveConst,
        repeated_description::RepeatedDescription,
        repeated_example::RepeatedExample,
        repeated_example_group_body::RepeatedExampleGroupBody,
        repeated_example_group_description::RepeatedExampleGroupDescription,
        repeated_include_example::RepeatedIncludeExample,
        repeated_subject_call::RepeatedSubjectCall,
        return_from_stub::ReturnFromStub,
        scattered_let::ScatteredLet,
        scattered_setup::ScatteredSetup,
        shared_context::SharedContext,
        shared_examples::SharedExamples,
        single_argument_message_chain::SingleArgumentMessageChain,
        skip_block_inside_example::SkipBlockInsideExample,
        sort_metadata::SortMetadata,
        spec_file_path_format::SpecFilePathFormat,
        spec_file_path_suffix::SpecFilePathSuffix,
        stubbed_mock::StubbedMock,
        subject_declaration::SubjectDeclaration,
        subject_stub::SubjectStub,
        undescriptive_literals_description::UndescriptiveLiteralsDescription,
        unspecified_exception::UnspecifiedException,
        variable_definition::VariableDefinition,
        variable_name::VariableName,
        verified_double_reference::VerifiedDoubleReference,
        verified_doubles::VerifiedDoubles,
        void_expect::VoidExpect,
        yield_cop::Yield,
    );
}

#[cfg(test)]
mod parity_tests;

mod subject_scopes;

mod ordering;

mod description;
