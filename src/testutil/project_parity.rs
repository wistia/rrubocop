use crate::cop::{CopConfig, CopRegistry};
use std::collections::HashMap;

fn check(cop: &str, source: &[u8], options: &str) {
    let mut values: HashMap<String, serde_yml::Value> =
        serde_yml::from_str(options).unwrap_or_default();
    values.insert("__RubocopVersion".into(), "1.65.1".into());
    values.insert(
        "TargetRubyVersion".into(),
        serde_yml::Value::Number(serde_yml::Number::from(3.4)),
    );
    let config = CopConfig {
        options: values,
        ..CopConfig::default()
    };
    super::assert_cop_no_offenses_full_with_config(
        CopRegistry::default_registry().get(cop).unwrap(),
        source,
        config,
    );
}

#[test]
fn layout_hash_alignment() {
    check(
        "Layout/HashAlignment",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/layout/hash_alignment/project_no_offense.rb"
        )),
        r#"ArgumentAlignmentStyle: with_fixed_indentation"#,
    );
}

#[test]
fn layout_array_alignment() {
    check(
        "Layout/ArrayAlignment",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/layout/array_alignment/project_no_offense.rb"
        )),
        r#"EnforcedStyle: with_fixed_indentation"#,
    );
}

#[test]
fn layout_space_inside_hash_literal_braces() {
    check(
        "Layout/SpaceInsideHashLiteralBraces",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/layout/space_inside_hash_literal_braces/project_no_offense.rb"
        )),
        r#"EnforcedStyle: no_space"#,
    );
}

#[test]
fn graphql_ordered_fields() {
    check(
        "GraphQL/OrderedFields",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/graphql/ordered_fields/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn style_arguments_forwarding() {
    check(
        "Style/ArgumentsForwarding",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/style/arguments_forwarding/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn style_percent_literal_delimiters() {
    check(
        "Style/PercentLiteralDelimiters",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/style/percent_literal_delimiters/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn lint_void() {
    check(
        "Lint/Void",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/void/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn style_frozen_string_literal_comment() {
    check(
        "Style/FrozenStringLiteralComment",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/style/frozen_string_literal_comment/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn lint_ordered_magic_comments() {
    check(
        "Lint/OrderedMagicComments",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/ordered_magic_comments/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn graphql_argument_name() {
    check(
        "GraphQL/ArgumentName",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/graphql/argument_name/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn naming_constant_name() {
    check(
        "Naming/ConstantName",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/naming/constant_name/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn lint_redundant_regexp_quantifiers() {
    check(
        "Lint/RedundantRegexpQuantifiers",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/redundant_regexp_quantifiers/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn layout_empty_lines() {
    check(
        "Layout/EmptyLines",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/layout/empty_lines/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn layout_trailing_whitespace() {
    check(
        "Layout/TrailingWhitespace",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/layout/trailing_whitespace/project_no_offense.rb"
        )),
        r#"AllowInHeredoc: true"#,
    );
}

#[test]
fn lint_missing_cop_enable_directive() {
    check(
        "Lint/MissingCopEnableDirective",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/missing_cop_enable_directive/project_no_offense.rb"
        )),
        r#"MaximumRangeSize: .inf"#,
    );
}

#[test]
fn rails_request_referer() {
    check(
        "Rails/RequestReferer",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/rails/request_referer/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn rails_find_each() {
    check(
        "Rails/FindEach",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/rails/find_each/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn style_redundant_self() {
    check(
        "Style/RedundantSelf",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/style/redundant_self/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn lint_format_parameter_mismatch() {
    check(
        "Lint/FormatParameterMismatch",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/format_parameter_mismatch/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn layout_space_around_keyword() {
    check(
        "Layout/SpaceAroundKeyword",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/layout/space_around_keyword/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn layout_empty_lines_around_access_modifier() {
    check(
        "Layout/EmptyLinesAroundAccessModifier",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/layout/empty_lines_around_access_modifier/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn layout_access_modifier_indentation() {
    check(
        "Layout/AccessModifierIndentation",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/layout/access_modifier_indentation/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn rspec_expect_actual() {
    check(
        "RSpec/ExpectActual",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/rspec/expect_actual/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn style_redundant_exception() {
    check(
        "Style/RedundantException",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/style/redundant_exception/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn layout_dot_position() {
    check(
        "Layout/DotPosition",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/layout/dot_position/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn lint_ineffective_access_modifier() {
    check(
        "Lint/IneffectiveAccessModifier",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/ineffective_access_modifier/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn rails_time_zone() {
    check(
        "Rails/TimeZone",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/rails/time_zone/project_no_offense.rb"
        )),
        r#"__RubocopRailsVersion: "2.26.0""#,
    );
}

#[test]
fn lint_constant_definition_in_block() {
    check(
        "Lint/ConstantDefinitionInBlock",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/constant_definition_in_block/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn rails_read_write_attribute() {
    check(
        "Rails/ReadWriteAttribute",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/rails/read_write_attribute/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn style_redundant_heredoc_delimiter_quotes() {
    check(
        "Style/RedundantHeredocDelimiterQuotes",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/style/redundant_heredoc_delimiter_quotes/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn lint_unreachable_code() {
    check(
        "Lint/UnreachableCode",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/unreachable_code/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn style_identical_conditional_branches() {
    check(
        "Style/IdenticalConditionalBranches",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/style/identical_conditional_branches/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn lint_useless_setter_call() {
    check(
        "Lint/UselessSetterCall",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/useless_setter_call/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn style_method_call_without_args_parentheses() {
    check(
        "Style/MethodCallWithoutArgsParentheses",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/style/method_call_without_args_parentheses/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn layout_indentation_consistency() {
    check(
        "Layout/IndentationConsistency",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/layout/indentation_consistency/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn layout_multiline_method_call_indentation() {
    check(
        "Layout/MultilineMethodCallIndentation",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/layout/multiline_method_call_indentation/project_no_offense.rb"
        )),
        r#"EnforcedStyle: indented"#,
    );
}

#[test]
fn style_safe_navigation() {
    check(
        "Style/SafeNavigation",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/style/safe_navigation/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn layout_comment_indentation() {
    check(
        "Layout/CommentIndentation",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/layout/comment_indentation/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn layout_empty_lines_around_arguments() {
    check(
        "Layout/EmptyLinesAroundArguments",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/layout/empty_lines_around_arguments/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn graphql_unused_argument() {
    check(
        "GraphQL/UnusedArgument",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/graphql/unused_argument/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn graphql_field_name() {
    check(
        "GraphQL/FieldName",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/graphql/field_name/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn layout_closing_parenthesis_indentation() {
    check(
        "Layout/ClosingParenthesisIndentation",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/layout/closing_parenthesis_indentation/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn lint_useless_assignment() {
    check(
        "Lint/UselessAssignment",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/useless_assignment/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn layout_multiline_operation_indentation() {
    check(
        "Layout/MultilineOperationIndentation",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/layout/multiline_operation_indentation/project_no_offense.rb"
        )),
        r#""#,
    );
}

#[test]
fn positive_controls_still_report_offenses() {
    let registry = CopRegistry::default_registry();
    let mut config = CopConfig::default();
    config.options.insert(
        "TargetRubyVersion".into(),
        serde_yml::Value::Number(serde_yml::Number::from(3.4)),
    );
    for &(name, source) in positive_cases() {
        let diagnostics =
            super::run_cop_full_with_config(registry.get(name).unwrap(), source, config.clone());
        assert!(
            !diagnostics.is_empty(),
            "positive control stopped reporting: {name}"
        );
    }
}

fn positive_cases() -> &'static [(&'static str, &'static [u8])] {
    &[
        ("Layout/HashAlignment", b"{\n  a: 1,\n    b: 2\n}\n"),
        ("Layout/ArrayAlignment", b"[first,\n    second]\n"),
        (
            "Style/ArgumentsForwarding",
            b"def process(**kwargs)\n  target(**kwargs)\nend\n",
        ),
        ("Style/SafeNavigation", b"value && value.name\n"),
        (
            "Style/IdenticalConditionalBranches",
            b"condition ? result : result\n",
        ),
        (
            "Layout/MultilineOperationIndentation",
            b"value = first +\nsecond\n",
        ),
        (
            "Rails/SkipsModelValidations",
            b"Record.update_all(active: true)\n",
        ),
        ("Security/Eval", b"eval(source)\n"),
        ("RSpec/Focus", b"fit 'works' do\nend\n"),
        (
            "GraphQL/OrderedFields",
            b"class Example\n  field :zebra, String\n  field :apple, String\nend\n",
        ),
        ("GraphQL/FieldName", b"field :badName, String\n"),
        (
            "Lint/IneffectiveAccessModifier",
            b"class Example\n  private\n  def self.helper; end\nend\n",
        ),
        ("Lint/UselessAssignment", b"def work\n  unused = 1\nend\n"),
        (
            "Lint/UnreachableCode",
            b"def work\n  raise Error\n  call\nend\n",
        ),
        ("Rails/ReadWriteAttribute", b"read_attribute(:name)\n"),
        ("Rails/RequestReferer", b"request.referrer\n"),
        ("Style/RedundantSelf", b"self.name\n"),
        ("Style/RedundantException", b"raise RuntimeError, message\n"),
        (
            "Lint/ConstantDefinitionInBlock",
            b"configure do\n  LIMIT = 5\nend\n",
        ),
    ]
}

#[test]
fn layout_heredoc_method_call_brace() {
    check(
        "Layout/MultilineMethodCallBraceLayout",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/layout/multiline_method_call_brace_layout/project_no_offense.rb"
        )),
        "",
    );
}
