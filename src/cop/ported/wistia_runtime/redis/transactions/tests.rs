use super::*;
use crate::{cop::CopConfig, parse::source::SourceFile};
use std::cell::Cell;

const CASES: &[&str] = &[
    "context.set('key', value)\n",
    "# multi and pipelined are only comments\ncontext.set('key', value)\n",
    "marker='multi'; context.set('key', value)\n",
    "redis.multi do |t|;t.set('a','v');t.set('b','v');end\n",
    "redis.pipelined { |t| t.set('a','v'); t.set('b','v', ex:60) }\n",
    "other.multi do |t|;t.set('a','v');marker='redis';end\n",
    "redis.multi do |t|\nt.set('a','v')\n# comment\nend\n",
    "redis.multi do |t|;items.each { |t| t.set('a','v') };noop;end\n",
    "REDIS.set('key', {ex:60})\n",
];

#[test]
fn marker_guard_keeps_existing_structural_transaction_and_payload_rules() {
    for text in CASES {
        compare(text);
    }
}
fn with_context(text: &str, check: impl FnOnce(&Context<'_>, Node<'_>)) {
    let source = SourceFile::from_bytes("redis.rb", text.as_bytes().to_vec());
    let tree = crate::parse::parse_ruby(&source).unwrap();
    let config = CopConfig::default();
    check(&Context::new(&source, &config), tree.root_node());
}
fn compare(text: &str) {
    with_context(text, |c, root| {
        let previous = Transactions::new(c, root);
        for call in descendants(root).filter(|n| is_call(*n) && c.method(*n) == "set") {
            let expected = !has_expiration(c, call)
                && (c.receiver(call).to_lowercase().contains("redis") || previous.contains(call));
            assert_eq!(!redis_expiry(c, call).is_empty(), expected, "{text}");
        }
    });
}

#[test]
fn ordinary_set_does_not_initialize_the_full_transaction_index() {
    with_context("context.set('key', value)", |c, root| {
        let call = descendants(root)
            .find(|n| is_call(*n) && c.method(*n) == "set")
            .unwrap();
        assert!(redis_expiry(c, call).is_empty());
        assert_index_uninitialized(c, call);
    });
}
fn assert_index_uninitialized(c: &Context<'_>, call: Node<'_>) {
    let initialized = Cell::new(false);
    c.source.cached(0, || {
        initialized.set(true);
        Transactions::new(c, call)
    });
    assert!(initialized.get());
}

#[test]
fn marker_cache_handles_short_sources_and_exact_supported_spellings() {
    for (text, expected) in [
        ("", false),
        ("set", false),
        ("MULTI", false),
        (".multi", true),
        (".pipelined", true),
        ("multiplication", true),
    ] {
        assert_eq!(
            Marker::new(&SourceFile::from_bytes(
                "redis.rb",
                text.as_bytes().to_vec()
            ))
            .present(),
            expected
        );
    }
}
