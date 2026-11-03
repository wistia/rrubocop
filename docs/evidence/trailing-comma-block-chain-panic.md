# Trailing-comma block-chain panic

The optimized integrated snapshot c9d11a4 panicked while scanning Wistia
310b8e42e91, in `spec/models/concerns/open_search_indexable_spec.rb:324–331`.
The command-block receiver ends with `end.at_least(:once)`; its final closing
parenthesis is byte 11416, while the unnormalized grouped argument ends at 11417.

Minimal source:

```ruby
expect(X).to receive(:x).and_wrap_original do |method, *args|
  method.call(*args)
end.at_least(:once)
```

Tree-sitter stores this actual invocation's parentheses inside a
`parenthesized_statements` child of `argument_list`. The old byte-prefix check
mistook that group itself for the last argument and sliced its end past the
closing delimiter. Pinned RuboCop 1.65.1 confirms this is a parenthesized call
with a Symbol argument. Genuine `foo (bar)` instead has no invocation delimiter
in the original AST.

The fix obtains delimiters from direct token children and unwraps the native
representation only when a complete grouped container is adjacent to the method
selector. Argument ranges then end at or before the actual closing token. It
preserves genuine grouping and ordinary/nested parenthesized call arguments;
there is no clamped slice or panic suppression.

Twenty-one original-backed controls cover all three trailing-comma styles,
command-block receivers, genuine grouping, ordinary calls and nested groups.
Two nondefault comma insertions remain explicit safety differences: original
corrections are Ruby-valid but produce native ERROR nodes in this command-block
receiver context. The warning/range remains, with correction declined. Generic
syntax recovery remains deferred.

All 12 scoped trailing-argument tests pass, including the complete previous
source/spec/safeguard corpus and the new range invariant. The rebuilt native and
original CLIs each inspect the actual Wistia file, exit 0 and report no offenses;
the native process no longer panics. This is a narrow isolated benchmark-unblock
checkpoint; the integrated branch will run its combined validation separately.
