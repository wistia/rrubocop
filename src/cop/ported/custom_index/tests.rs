use super::*;
use crate::{cop::CopConfig, parse::source::SourceFile};

const SOURCE: &str = "class OuterPolicy\n attribute :nested\n attribute(:nested) { nil }\n class Inner\n def nested; end\n def show?; end\n end\n class << self\n attribute :own\n def own; end\n end\n class Scope < ApplicationPolicy::Scope; end\n disable_scoping!\nend\nclass Worker < BaseWorker\n sidekiq_options queue: :work\n def perform; work; end\n class Nested\n include Sidekiq::Worker\n use_default_queue!\n end\nend\nclass Single\n include Sidekiq::Worker\nend\nRSpec.describe 'x' do\n class Dummy; @value; end\nend\n@root\n";
fn scope(n: Node<'_>) -> Option<Node<'_>> {
    ancestors(n).find(|n| matches!(n.kind(), "class" | "module" | "singleton_class"))
}
fn old_method(c: &Context<'_>, n: Node<'_>, name: &str, strict: bool) -> bool {
    let owner = if strict { scope(n) } else { n.parent() };
    owner.is_some_and(|owner| {
        descendants(owner).any(|m| {
            m.kind() == "method" && c.method(m) == name && (!strict || scope(m) == Some(owner))
        })
    })
}
#[test]
fn method_index_keeps_distinct_owner_and_parent_subtree_policies_across_parses() {
    let source = SourceFile::from_bytes("example.rb", SOURCE.as_bytes().to_vec());
    let config = CopConfig::default();
    let c = Context::new(&source, &config);
    let first = crate::parse::parse_ruby(&source).unwrap();
    check_methods(&c, first.root_node());
    let second = crate::parse::parse_ruby(&source).unwrap();
    check_methods(&c, second.root_node());
}
fn check_methods(c: &Context<'_>, root: Node<'_>) {
    let summary = index(c, root);
    for call in descendants(root).filter(|n| is_call(*n) && c.method(*n) == "attribute") {
        for strict in [true, false] {
            for name in ["nested", "own", "missing"] {
                assert_eq!(
                    summary.method(call, name, strict),
                    old_method(c, call, name, strict),
                    "{} {name} {strict}",
                    c.text(call)
                );
            }
        }
    }
}
#[test]
fn queue_and_policy_summaries_keep_nested_owners_and_single_statement_exclusion() {
    let source = SourceFile::from_bytes("example.rb", SOURCE.as_bytes().to_vec());
    let config = CopConfig::default();
    let c = Context::new(&source, &config);
    let tree = crate::parse::parse_ruby(&source).unwrap();
    let summary = index(&c, tree.root_node());
    check_queues(&c, tree.root_node(), &summary);
    check_policy(&c, tree.root_node(), &summary);
}
fn check_policy(c: &Context<'_>, root: Node<'_>, summary: &Index) {
    let show = descendants(root)
        .find(|n| n.kind() == "method" && c.method(*n) == "show?")
        .unwrap();
    assert_eq!(summary.parent_policy_disabled(show), Some(true));
}
fn old_queue(c: &Context<'_>, class: Node<'_>) -> (bool, bool) {
    let calls = old_calls(class);
    (
        calls.iter().any(|n| includes(c, *n)),
        calls.iter().any(|n| declared(c, *n)),
    )
}
fn includes(c: &Context<'_>, n: Node<'_>) -> bool {
    c.method(n) == "include" && args(n).iter().any(|a| c.text(*a) == "Sidekiq::Worker")
}
fn declared(c: &Context<'_>, n: Node<'_>) -> bool {
    c.method(n) == "use_default_queue!"
        || c.method(n) == "sidekiq_options" && c.pair_value(n, "queue").is_some()
}
fn old_calls(class: Node<'_>) -> Vec<Node<'_>> {
    let excluded = old_excluded(class);
    descendants(class)
        .filter(|n| {
            Some(*n) != excluded
                && is_call(*n)
                && ancestors(*n).find(|p| matches!(p.kind(), "class" | "module")) == Some(class)
        })
        .collect()
}
fn check_queues(c: &Context<'_>, root: Node<'_>, summary: &Index) {
    for class in descendants(root).filter(|n| n.kind() == "class") {
        assert_eq!(
            summary.worker_queue(class),
            old_queue(c, class),
            "{}",
            c.text(class)
        );
    }
}

#[test]
fn ancestor_memo_keeps_rspec_and_class_context_without_leaking_to_later_nodes() {
    let source = SourceFile::from_bytes("example.rb", SOURCE.as_bytes().to_vec());
    let config = CopConfig::default();
    let c = Context::new(&source, &config);
    let tree = crate::parse::parse_ruby(&source).unwrap();
    for n in descendants(tree.root_node()).filter(|n| n.kind() == "instance_variable") {
        assert_eq!(in_class(&c, n), ancestors(n).any(|p| p.kind() == "class"));
        assert_eq!(in_spec(&c, n), c.text(n) == "@value");
    }
}

fn old_excluded(class: Node<'_>) -> Option<Node<'_>> {
    let statements = body(class).map(statements).unwrap_or_default();
    (statements.len() == 1).then(|| statements[0])
}
