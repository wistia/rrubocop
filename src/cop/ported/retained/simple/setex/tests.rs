use super::*;
use crate::cop::CopConfig;
use crate::parse::source::SourceFile;

fn inspect(code: &str) -> Finding {
    let source = SourceFile::from_bytes("probe.rb", code.as_bytes().to_vec());
    let tree = crate::parse::parse_ruby(&source).unwrap();
    let c = Context {
        source: &source,
        config: &CopConfig::default(),
    };
    setex(&c, candidate(&c, tree.root_node())).remove(0)
}
fn candidate<'n>(c: &Context<'_>, n: Node<'n>) -> Node<'n> {
    descendants(n)
        .into_iter()
        .find(|n| is_call(*n) && c.method(*n) == "setex")
        .unwrap()
}

fn corrected(code: &str, f: Finding) -> String {
    let mut edits = f.additional;
    edits.push(f.edit.unwrap());
    edits.sort_by_key(|e| std::cmp::Reverse(e.0));
    let mut result = code.to_owned();
    for (start, end, text) in edits {
        result.replace_range(start..end, &text);
    }
    result
}

#[test]
fn stable_literals_and_lexical_locals_can_be_swapped() {
    for (source, expected) in [
        (
            "redis.setex(key, 60, \"payload\")\n",
            "redis.set(key, \"payload\", ex: 60)\n",
        ),
        (
            "def f(key, ttl, value)\n redis.setex(key, ttl, value)\nend\n",
            "def f(key, ttl, value)\n redis.set(key, value, ex: ttl)\nend\n",
        ),
        (
            "ttl = 60; value = :value\nredis.setex(key, ttl, value, &callback)\n",
            "ttl = 60; value = :value\nredis.set(key, value, ex: ttl, &callback)\n",
        ),
    ] {
        assert_eq!(corrected(source, inspect(source)), expected);
    }
}

#[test]
fn effectful_calls_and_uncertain_argument_shapes_are_not_corrected() {
    for source in [
        "events = [60, 1234]\nredis.setex(key, events.shift, events.shift)\n",
        "redis.setex(key, ttl, value)\n",
        "redis.setex(key, *ttl_and_value, 1234)\n",
        "redis.setex(key, 60, a: 1, b: 2)\n",
        "redis.setex(key, 60, {a: 1})\n",
        "redis.setex(key, 60, \"#{payload}\")\n",
        "redis.setex(key, TTL, VALUE)\n",
        "redis.setex(key, 60, 1234, extra)\n",
        "redis.setex(key, <<~TTL, <<~VALUE)\n 60\nTTL\n payload\nVALUE\n",
    ] {
        let f = inspect(source);
        assert!(f.edit.is_none() && f.additional.is_empty(), "{source}");
    }
}
