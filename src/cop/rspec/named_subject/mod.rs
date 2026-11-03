//! RSpec/NamedSubject — name `subject` when an example references it.

use tree_sitter::Node;

use crate::cop::rspec::helpers::{RSPEC_INCLUDE, bare_rspec_call, call_block};
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, method_node, node_bytes,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

use crate::cop::rspec::separation::numbered_block;

pub struct NamedSubject;

const MSG: &str = "Name your test subject if you need to reference it explicitly.";

const EXAMPLES: &[&[u8]] = &[
    b"it",
    b"specify",
    b"example",
    b"scenario",
    b"its",
    b"fit",
    b"fspecify",
    b"fexample",
    b"fscenario",
    b"focus",
    b"xit",
    b"xspecify",
    b"xexample",
    b"xscenario",
    b"skip",
    b"pending",
];

const HOOKS: &[&[u8]] = &[
    b"prepend_before",
    b"before",
    b"append_before",
    b"around",
    b"prepend_after",
    b"after",
    b"append_after",
];

fn bare_method<'a>(source: &'a SourceFile, node: Node<'_>) -> Option<&'a [u8]> {
    if !matches!(node.kind(), "call" | "command" | "command_call") || call_receiver(node).is_some()
    {
        return None;
    }
    call_method_name(source, node)
}

fn is_example_or_hook(source: &SourceFile, node: Node<'_>) -> bool {
    if !matches!(node.kind(), "do_block" | "block") {
        return false;
    }
    node.parent()
        .filter(|call| !numbered_block(source, *call))
        .and_then(|call| bare_method(source, call))
        .is_some_and(|name| EXAMPLES.contains(&name) || HOOKS.contains(&name))
}

fn is_shared_examples(source: &SourceFile, node: Node<'_>) -> bool {
    matches!(node.kind(), "do_block" | "block")
        && node.parent().is_some_and(|call| {
            !numbered_block(source, call)
                && bare_rspec_call(source, call)
                    .is_some_and(|name| matches!(name, b"shared_examples" | b"shared_examples_for"))
        })
}

fn in_shared_examples(source: &SourceFile, node: Node<'_>) -> bool {
    let mut parent = node.parent();
    while let Some(cur) = parent {
        if is_shared_examples(source, cur) {
            return true;
        }
        parent = cur.parent();
    }
    false
}

fn is_method_name(node: Node<'_>) -> bool {
    node.parent()
        .and_then(method_node)
        .is_some_and(|name| name.id() == node.id())
}

fn bare_subject_call<'a>(source: &SourceFile, node: Node<'a>) -> Option<Node<'a>> {
    if bare_method(source, node) != Some(b"subject") || !argument_nodes(node).is_empty() {
        return None;
    }
    method_node(node)
}

fn bare_subject_read<'a>(source: &SourceFile, node: Node<'a>) -> Option<Node<'a>> {
    if node.kind() == "hash_key_symbol"
        && node_bytes(source, node) == b"subject"
        && node.parent().is_some_and(|pair| {
            pair.kind() == "pair" && pair.child_by_field_name("value").is_none()
        })
        && !crate::cop::ported::local_identifier(source, node)
    {
        return Some(node);
    }
    if node.kind() != "identifier"
        || node_bytes(source, node) != b"subject"
        || is_method_name(node)
        || !crate::cop::ported::bare_call_identifier(source, node)
    {
        return None;
    }
    Some(node)
}

fn statements<'a>(body: Node<'a>) -> Vec<Node<'a>> {
    if matches!(body.kind(), "body_statement" | "block_body") {
        let mut cur = body.walk();
        return body
            .named_children(&mut cur)
            .filter(|child| child.kind() != "comment")
            .collect();
    }
    vec![body]
}

fn subject_def_named(source: &SourceFile, node: Node<'_>) -> Option<bool> {
    let name = bare_method(source, node)?;
    if name != b"subject" && name != b"subject!" {
        return None;
    }
    if call_block(node).is_none() || numbered_block(source, node) {
        return None;
    }
    Some(!argument_nodes(node).is_empty())
}

fn uncached_subject_named_in(source: &SourceFile, block: Node<'_>) -> Option<bool> {
    let body = block.child_by_field_name("body")?;
    let statements = statements(body);
    let candidates = if statements.len() == 1 {
        let single = statements[0];
        if let Some(block) = call_block(single) {
            block
                .child_by_field_name("body")
                .map(|body| {
                    let nodes = crate::cop::rspec::parity_helpers::code_children(body);
                    if nodes.len() == 1 { nodes } else { vec![body] }
                })
                .unwrap_or_default()
        } else if single.kind() == "call" {
            let mut children: Vec<_> = call_receiver(single).into_iter().collect();
            children.extend(argument_nodes(single));
            children
        } else {
            crate::cop::rspec::parity_helpers::code_children(single)
        }
    } else {
        statements
    };
    candidates
        .into_iter()
        .find_map(|candidate| subject_def_named(source, candidate))
}

#[derive(Clone)]
struct SubjectNamed(Option<bool>);
fn subject_named_in(source: &SourceFile, block: Node<'_>) -> Option<bool> {
    super::cache::node_fact(source, block, || {
        SubjectNamed(uncached_subject_named_in(source, block))
    })
    .0
}

fn nearest_named(source: &SourceFile, node: Node<'_>) -> bool {
    let mut ancestor = node.parent();
    while let Some(parent) = ancestor {
        if !numbered_block(source, parent) {
            if let Some(named) =
                call_block(parent).and_then(|block| subject_named_in(source, block))
            {
                return named;
            }
        }
        ancestor = parent.parent();
    }
    false
}

fn allowed(style: &str, source: &SourceFile, node: Node<'_>) -> bool {
    match style {
        "always" => false,
        "named_only" => !nearest_named(source, node),
        _ => true,
    }
}

fn push_offense(
    cop: &NamedSubject,
    source: &SourceFile,
    node: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let (line, col) = source.offset_to_line_col(node.start_byte());
    if diagnostics.iter().any(|diagnostic| {
        diagnostic.cop_name == cop.name()
            && diagnostic.location.line == line
            && diagnostic.location.column == col
    }) {
        return;
    }
    let mut diag = cop.diagnostic(source, line, col, MSG.to_string());
    diag.highlight_length = node.end_byte().saturating_sub(node.start_byte()).max(1);
    diagnostics.push(diag);
}

fn report_usages(
    cop: &NamedSubject,
    source: &SourceFile,
    block: Node<'_>,
    style: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut stack = vec![block.parent().unwrap_or(block)];
    while let Some(node) = stack.pop() {
        if let Some(target) =
            bare_subject_call(source, node).or_else(|| bare_subject_read(source, node))
            && !allowed(style, source, target)
        {
            push_offense(cop, source, target, diagnostics);
        }
        let mut cur = node.walk();
        for child in node.children(&mut cur) {
            stack.push(child);
        }
    }
}

impl Cop for NamedSubject {
    fn name(&self) -> &'static str {
        "RSpec/NamedSubject"
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["do_block", "block"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if !is_example_or_hook(source, node) {
            return;
        }
        if config.get_bool("IgnoreSharedExamples", true) && in_shared_examples(source, node) {
            return;
        }
        report_usages(
            self,
            source,
            node,
            config.get_str("EnforcedStyle", "always"),
            diagnostics,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(NamedSubject, "cops/rspec/named_subject");

    fn with_style(style: &str, ignore_shared: bool) -> CopConfig {
        let mut config = CopConfig::default();
        config.options.insert(
            "EnforcedStyle".into(),
            serde_yml::Value::String(style.into()),
        );
        config.options.insert(
            "IgnoreSharedExamples".into(),
            serde_yml::Value::Bool(ignore_shared),
        );
        config
    }

    #[test]
    fn named_only_ignores_unnamed_subject() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &NamedSubject,
            b"RSpec.describe User do
  subject { described_class.new }
  it 'is a User' do
    expect(subject).to be_a(User)
  end
  subject! { described_class.new }
  it 'is valid' do
    expect(subject.valid?).to be(true)
  end
end
",
            with_style("named_only", true),
        );
    }

    #[test]
    fn named_only_flags_named_subject() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &NamedSubject,
            b"RSpec.describe User do
  subject(:new_user) { described_class.new }
  it 'is a User' do
    expect(subject).to be_a(User)
           ^^^^^^^ RSpec/NamedSubject: Name your test subject if you need to reference it explicitly.
  end
  subject!(:new_user) { described_class.new }
  it 'is valid' do
    expect(subject.valid?).to be(true)
           ^^^^^^^ RSpec/NamedSubject: Name your test subject if you need to reference it explicitly.
  end
end
",
            with_style("named_only", true),
        );
    }

    #[test]
    fn named_only_uses_closest_declaration() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &NamedSubject,
            b"RSpec.describe User do
  subject(:user) { described_class.new }
  describe 'age' do
    subject { user.age }
    it 'allows driving' do
      expect(subject.can_drive_a_car).to be(true)
    end
  end
end
it 'has no body' do
  subject.each do
  end
end
",
            with_style("named_only", true),
        );
    }

    #[test]
    fn flags_bare_subject_in_nested_example() {
        crate::testutil::assert_cop_offenses_full(
            &NamedSubject,
            b"RSpec.describe Caching::Base, :cache do
  describe 'metrics' do
    subject { instance.call }

    context 'fresh' do
      it do
        Rails.cache.write(cache_key, { data: 'cached' })

        subject
        ^^^^^^^ RSpec/NamedSubject: Name your test subject if you need to reference it explicitly.

        expect(Telemetry::Metrics::Cache.requests).to have_recorded(1)
      end
    end
  end
end
",
        );
    }

    #[test]
    fn reports_shared_examples_when_not_ignored() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &NamedSubject,
            b"RSpec.describe User do
  shared_examples_for 'a user' do
    it 'is a User' do
      expect(subject).to be_a(User)
             ^^^^^^^ RSpec/NamedSubject: Name your test subject if you need to reference it explicitly.
    end
  end
end
",
            with_style("always", false),
        );
    }
}
