# RSpec performance coverage inventory

Snapshot: `2db3f585334e669e5b1466ded9ffadb1e7dff0ec`.

106/106 implemented registrations screened; 11454 runtime lines across 106 rule files, plus seven RSpec helper files. No Minitest or extra RSpec-factory namespace implementations; FactoryBot belongs to framework review.

Enabled column comes from pinned Wistia Ruby inventory: yes96/no8/unknown2. This is not a runtime eligibility assertion. All rows include common dispatch/helper cost review.

| Cop | Source callback line | Enabled | Specific findings |
|---|---|---|---|
| RSpec/AnyInstance | `src/cop/rspec/any_instance.rs:24` | no | local inspection, F9 |
| RSpec/AroundBlock | `src/cop/rspec/around_block.rs:21` | yes | local inspection, F9 |
| RSpec/Be | `src/cop/rspec/be.rs:20` | yes | local inspection, F9 |
| RSpec/BeEmpty | `src/cop/rspec/be_empty.rs:24` | yes | local inspection, F9 |
| RSpec/BeEq | `src/cop/rspec/be_eq.rs:26` | yes | local inspection, F9 |
| RSpec/BeEql | `src/cop/rspec/be_eql.rs:26` | yes | local inspection, F9 |
| RSpec/BeNil | `src/cop/rspec/be_nil.rs:24` | yes | local inspection, F9 |
| RSpec/BeforeAfterAll | `src/cop/rspec/before_after_all.rs:20` | yes | local inspection, F9 |
| RSpec/ChangeByZero | `src/cop/rspec/change_by_zero.rs:121` | yes | local inspection, F9 |
| RSpec/ClassCheck | `src/cop/rspec/class_check.rs:24` | yes | local inspection, F9 |
| RSpec/ContainExactly | `src/cop/rspec/contain_exactly.rs:24` | yes | local inspection, F9 |
| RSpec/ContextMethod | `src/cop/rspec/context_method.rs:23` | yes | local inspection, F9 |
| RSpec/ContextWording | `src/cop/rspec/context_wording.rs:21` | yes | F7, F9 |
| RSpec/DescribeClass | `src/cop/rspec/describe_class.rs:20` | yes | F5, F9 |
| RSpec/DescribeMethod | `src/cop/rspec/describe_method.rs:20` | yes | F5, F9 |
| RSpec/DescribeSymbol | `src/cop/rspec/describe_symbol.rs:20` | yes | local inspection, F9 |
| RSpec/DescribedClass | `src/cop/rspec/described_class.rs:180` | no | F5, F9 |
| RSpec/DuplicatedMetadata | `src/cop/rspec/duplicated_metadata.rs:24` | yes | local inspection, F9 |
| RSpec/EmptyExampleGroup | `src/cop/rspec/empty_example_group.rs:154` | yes | local inspection, F9 |
| RSpec/EmptyHook | `src/cop/rspec/empty_hook.rs:24` | yes | local inspection, F9 |
| RSpec/EmptyLineAfterExample | `src/cop/rspec/empty_line_after_example.rs:25` | yes | F6, F9 |
| RSpec/EmptyLineAfterExampleGroup | `src/cop/rspec/empty_line_after_example_group.rs:24` | yes | F6, F9 |
| RSpec/EmptyLineAfterFinalLet | `src/cop/rspec/empty_line_after_final_let.rs:25` | yes | F6, F9 |
| RSpec/EmptyLineAfterHook | `src/cop/rspec/empty_line_after_hook.rs:25` | yes | F6, F9 |
| RSpec/EmptyLineAfterSubject | `src/cop/rspec/empty_line_after_subject.rs:25` | yes | F6, F9 |
| RSpec/EmptyMetadata | `src/cop/rspec/empty_metadata.rs:24` | yes | local inspection, F9 |
| RSpec/EmptyOutput | `src/cop/rspec/empty_output.rs:24` | yes | local inspection, F9 |
| RSpec/Eq | `src/cop/rspec/eq.rs:24` | yes | local inspection, F9 |
| RSpec/ExampleLength | `src/cop/rspec/example_length.rs:28` | no | local inspection, F9 |
| RSpec/ExampleWithoutDescription | `src/cop/rspec/example_without_description.rs:21` | yes | local inspection, F9 |
| RSpec/ExampleWording | `src/cop/rspec/example_wording.rs:128` | yes | F1,F8, F9 |
| RSpec/ExcessiveDocstringSpacing | `src/cop/rspec/excessive_docstring_spacing.rs:65` | yes | F1,F8, F9 |
| RSpec/ExpectActual | `src/cop/rspec/expect_actual.rs:70` | yes | local inspection, routed |
| RSpec/ExpectChange | `src/cop/rspec/expect_change.rs:154` | yes | local inspection, F9 |
| RSpec/ExpectInHook | `src/cop/rspec/expect_in_hook.rs:21` | yes | F5, F9 |
| RSpec/ExpectInLet | `src/cop/rspec/expect_in_let.rs:21` | yes | F5, F9 |
| RSpec/ExpectOutput | `src/cop/rspec/expect_output.rs:21` | yes | F5, F9 |
| RSpec/Focus | `src/cop/rspec/focus.rs:24` | yes | local inspection, F9 |
| RSpec/HookArgument | `src/cop/rspec/hook_argument.rs:24` | yes | local inspection, F9 |
| RSpec/HooksBeforeExamples | `src/cop/rspec/hooks_before_examples.rs:23` | yes | F6, F9 |
| RSpec/IdenticalEqualityAssertion | `src/cop/rspec/identical_equality_assertion.rs:21` | yes | local inspection, F9 |
| RSpec/ImplicitBlockExpectation | `src/cop/rspec/implicit_block_expectation.rs:41` | yes | F3, F9 |
| RSpec/ImplicitExpect | `src/cop/rspec/implicit_expect.rs:24` | yes | local inspection, F9 |
| RSpec/ImplicitSubject | `src/cop/rspec/implicit_subject.rs:24` | yes | F5, F9 |
| RSpec/IndexedLet | `src/cop/rspec/indexed_let.rs:170` | no | F7, F9 |
| RSpec/InstanceSpy | `src/cop/rspec/instance_spy.rs:25` | yes | F3,F8, F9 |
| RSpec/InstanceVariable | `src/cop/rspec/instance_variable.rs:93` | no | F5, F9 |
| RSpec/IsExpectedSpecify | `src/cop/rspec/is_expected_specify.rs:25` | yes | local inspection, F9 |
| RSpec/ItBehavesLike | `src/cop/rspec/it_behaves_like.rs:23` | yes | local inspection, F9 |
| RSpec/IteratedExpectation | `src/cop/rspec/iterated_expectation.rs:75` | yes | local inspection, F9 |
| RSpec/LeadingSubject | `src/cop/rspec/leading_subject.rs:24` | yes | F6, F9 |
| RSpec/LeakyConstantDeclaration | `src/cop/rspec/leaky_constant_declaration.rs:41` | yes | F5, F9 |
| RSpec/LetBeforeExamples | `src/cop/rspec/let_before_examples.rs:23` | yes | F6, F9 |
| RSpec/LetSetup | `src/cop/rspec/let_setup.rs:36` | yes | F3, F9 |
| RSpec/MatchArray | `src/cop/rspec/match_array.rs:24` | yes | local inspection, F9 |
| RSpec/MessageChain | `src/cop/rspec/message_chain.rs:21` | yes | local inspection, F9 |
| RSpec/MessageSpies | `src/cop/rspec/message_spies.rs:20` | yes | local inspection, F9 |
| RSpec/MetadataStyle | `src/cop/rspec/metadata_style.rs:154` | yes | local inspection, F9 |
| RSpec/MissingExampleGroupArgument | `src/cop/rspec/missing_example_group_argument.rs:20` | yes | local inspection, F9 |
| RSpec/MissingExpectationTargetMethod | `src/cop/rspec/missing_expectation_target_method.rs:21` | yes | local inspection, F9 |
| RSpec/MultipleDescribes | `src/cop/rspec/multiple_describes.rs:20` | yes | F5, F9 |
| RSpec/MultipleExpectations | `src/cop/rspec/multiple_expectations.rs:154` | no | local inspection, F9 |
| RSpec/MultipleMemoizedHelpers | `src/cop/rspec/multiple_memoized_helpers.rs:159` | no | F5, F9 |
| RSpec/MultipleSubjects | `src/cop/rspec/multiple_subjects.rs:59` | yes | F5, F9 |
| RSpec/NamedSubject | `src/cop/rspec/named_subject/mod.rs:246` | yes | F3,F5, F9 |
| RSpec/NestedGroups | `src/cop/rspec/nested_groups.rs:24` | no | F5, F9 |
| RSpec/NoExpectationExample | `src/cop/rspec/no_expectation_example.rs:21` | yes | F7, F9 |
| RSpec/NotToNot | `src/cop/rspec/not_to_not.rs:23` | yes | local inspection, F9 |
| RSpec/OverwritingSetup | `src/cop/rspec/overwriting_setup.rs:21` | yes | local inspection, F9 |
| RSpec/PendingWithoutReason | `src/cop/rspec/pending_without_reason.rs:100` | yes | local inspection, F9 |
| RSpec/PredicateMatcher | `src/cop/rspec/predicate_matcher.rs:140` | yes | local inspection, F9 |
| RSpec/ReceiveCounts | `src/cop/rspec/receive_counts.rs:24` | yes | local inspection, F9 |
| RSpec/ReceiveMessages | `src/cop/rspec/receive_messages.rs:233` | yes | F4, F9 |
| RSpec/ReceiveNever | `src/cop/rspec/receive_never.rs:24` | yes | local inspection, F9 |
| RSpec/RedundantAround | `src/cop/rspec/redundant_around.rs:25` | yes | local inspection, F9 |
| RSpec/RedundantPredicateMatcher | `src/cop/rspec/redundant_predicate_matcher.rs:23` | yes | local inspection, F9 |
| RSpec/RemoveConst | `src/cop/rspec/remove_const.rs:20` | yes | local inspection, F9 |
| RSpec/RepeatedDescription | `src/cop/rspec/repeated_description.rs:20` | yes | F4, F9 |
| RSpec/RepeatedExample | `src/cop/rspec/repeated_example.rs:31` | yes | F4, F9 |
| RSpec/RepeatedExampleGroupBody | `src/cop/rspec/repeated_example_group_body.rs:61` | yes | F4, F9 |
| RSpec/RepeatedExampleGroupDescription | `src/cop/rspec/repeated_example_group_description.rs:45` | yes | F4, F9 |
| RSpec/RepeatedIncludeExample | `src/cop/rspec/repeated_include_example.rs:29` | yes | F4, F9 |
| RSpec/RepeatedSubjectCall | `src/cop/rspec/repeated_subject_call.rs:87` | yes | F5, F9 |
| RSpec/ReturnFromStub | `src/cop/rspec/return_from_stub.rs:48` | yes | F2, F9 |
| RSpec/ScatteredLet | `src/cop/rspec/scattered_let.rs:23` | yes | F6, F9 |
| RSpec/ScatteredSetup | `src/cop/rspec/scattered_setup.rs:305` | yes | F4, F9 |
| RSpec/SharedContext | `src/cop/rspec/shared_context.rs:27` | yes | local inspection, F9 |
| RSpec/SharedExamples | `src/cop/rspec/shared_examples.rs:24` | yes | local inspection, F9 |
| RSpec/SingleArgumentMessageChain | `src/cop/rspec/single_argument_message_chain.rs:57` | yes | local inspection, F9 |
| RSpec/SkipBlockInsideExample | `src/cop/rspec/skip_block_inside_example.rs:21` | yes | F5, F9 |
| RSpec/SortMetadata | `src/cop/rspec/sort_metadata.rs:25` | yes | F8, F9 |
| RSpec/SpecFilePathFormat | `src/cop/rspec/spec_file_path_format.rs:85` | yes | F5,F7, F9 |
| RSpec/SpecFilePathSuffix | `src/cop/rspec/spec_file_path_suffix.rs:18` | yes | F5, F9 |
| RSpec/StubbedMock | `src/cop/rspec/stubbed_mock.rs:104` | yes | local inspection, F9 |
| RSpec/SubjectDeclaration | `src/cop/rspec/subject_declaration.rs:21` | yes | local inspection, F9 |
| RSpec/SubjectStub | `src/cop/rspec/subject_stub.rs:109` | yes | F5, F9 |
| RSpec/UndescriptiveLiteralsDescription | `src/cop/rspec/undescriptive_literals_description.rs:20` | yes | local inspection, F9 |
| RSpec/UnspecifiedException | `src/cop/rspec/unspecified_exception.rs:21` | yes | F5, F9 |
| RSpec/VariableDefinition | `src/cop/rspec/variable_definition.rs:24` | yes | F8, F9 |
| RSpec/VariableName | `src/cop/rspec/variable_name.rs:21` | yes | F7, F9 |
| RSpec/VerifiedDoubleReference | `src/cop/rspec/verified_double_reference.rs:27` | yes | F8, F9 |
| RSpec/VerifiedDoubles | `src/cop/rspec/verified_doubles.rs:21` | yes | local inspection, F9 |
| RSpec/VoidExpect | `src/cop/rspec/void_expect.rs:20` | yes | F5, F9 |
| RSpec/Yield | `src/cop/rspec/yield_cop.rs:40` | yes | local inspection, F9 |
| RSpecRails/HttpStatus | `src/cop/rspec_rails/http_status.rs:41` | unknown | local inspection, routed |
| RSpecRails/InferredSpecType | `src/cop/rspec_rails/inferred_spec_type.rs:148` | unknown | F8, F9 |
