# Raw argument grouping

Imported Performance's shared `ruby_arguments` implementation from `ba767c5`, with a thin `from_nodes(iterator)` entry point around the unchanged grouping loop. Both entry points group adjacent implicit Pair/keyword-splat nodes into one Hash argument; explicit Hash nodes retain braces and ordinary nodes retain their own source spans. Unary +Hash remains an ordinary unary-send argument, not a Hash.

The pinned RuboCop 1.65.1 property fixture records 26 authored ordinary-send observations: empty/single/multiple arguments, explicit/implicit/empty hashes, keyword spreads/omission, block passes, blocks, arrays, unary signs, forwarding, UTF-8, comments and delayed heredocs. Both entry points match original byte spans, Hash/braces flags and child counts. Regeneration is byte-identical; the Ruby generator passes default pinned lint.

This is raw keyword grouping, not a complete ordinary-call argument ownership normalizer. Parser-reinterpreted commands such as `outer(foo +1, other)` require the separately reviewed command ownership adapter. Signed command argument starts and original UnarySend/Hash roles can require caller span/type overrides; `from_nodes` does not silently recover them or parse error nodes.
