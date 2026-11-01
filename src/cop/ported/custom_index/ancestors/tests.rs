use super::*;
use crate::{
    cop::CopConfig,
    parse::{parse_ruby, source::SourceFile},
};

fn expected(c: &Context<'_>, n: Node<'_>) -> Flags {
    let mut flags = Flags::default();
    let mut parent = n.parent();
    while let Some(n) = parent {
        flags = including_self(c, n, flags);
        parent = n.parent();
    }
    flags
}

#[test]
fn adaptive_index_preserves_strict_ancestry_across_queries_and_parses() {
    let text = "class Outer\nRSpec.describe 'x' do\nclass Inner; @nested; end\nend\nend\nmodule M\ndescribe('other') { @spec }\nother.describe('not spec') { @plain }\nend\n::RSpec.describe('rooted') { @rooted }\nclass << self; @outside_singleton; end\nclass C; class << self; @inside_singleton; end; end\n@root\n";
    let source = SourceFile::from_bytes("probe.rb", text.as_bytes().to_vec());
    let config = CopConfig::default();
    let c = Context::new(&source, &config);
    for _ in 0..2 {
        let tree = parse_ruby(&source).unwrap();
        check_nodes(&c, tree.root_node());
    }
    assert!(
        source
            .cached(0, Contexts::default)
            .0
            .lock()
            .unwrap()
            .indexed
    );
}

#[test]
fn repeated_sparse_queries_stay_lazy_and_reuse_their_strict_flags() {
    let source = SourceFile::from_bytes("probe.rb", b"class Outer; @x; end\n".to_vec());
    let tree = parse_ruby(&source).unwrap();
    let config = CopConfig::default();
    let c = Context::new(&source, &config);
    let n = descendants(tree.root_node())
        .find(|n| n.kind() == "instance_variable")
        .unwrap();
    for _ in 0..100 {
        assert!(in_class(&c, n));
        assert!(!in_spec(&c, n));
    }
    assert_lazy(&source);
}

fn check_nodes(c: &Context<'_>, root: Node<'_>) {
    let nodes: Vec<_> = descendants(root).collect();
    for n in &nodes {
        assert_eq!(
            ancestors(c, *n),
            expected(c, *n),
            "{} {}",
            n.kind(),
            c.text(*n)
        );
    }
    for n in nodes.into_iter().rev() {
        assert_eq!(ancestors(c, n), expected(c, n));
    }
}

fn assert_lazy(source: &SourceFile) {
    let cache = source.cached(0, Contexts::default);
    let state = cache.0.lock().unwrap();
    assert!(!state.indexed);
    assert_eq!(state.queries.len(), 1);
}

#[test]
fn deep_many_query_contexts_switch_to_one_index_without_losing_scope() {
    let mut text = String::from("module Outer\n");
    for depth in 0..600 {
        text.push_str(&format!("def m{depth}\n@x\n"));
    }
    text.push_str(&"end\n".repeat(601));
    let source = SourceFile::from_bytes("probe.rb", text.into_bytes());
    let tree = parse_ruby(&source).unwrap();
    let config = CopConfig::default();
    let c = Context::new(&source, &config);
    assert_outside(&c, tree.root_node());
    assert!(
        source
            .cached(0, Contexts::default)
            .0
            .lock()
            .unwrap()
            .indexed
    );
}
fn assert_outside(c: &Context<'_>, root: Node<'_>) {
    for n in descendants(root).filter(|n| n.kind() == "instance_variable") {
        assert!(!in_class(c, n));
        assert!(!in_spec(c, n));
    }
}
