# UselessTimes correction policy

`Lint/UselessTimes` remains enabled with the same diagnostics, locations, and literal-count matching. It no longer offers corrections in either safe or unsafe correction mode. This supersedes the earlier policy that allowed selected `-A` rewrites after checking statement context and block scopes.

Ruby's `Integer#times` returns its receiver when its block completes normally. Replacing `1.times { :value }` with its body changes a method's return from `1` to `:value`; deleting zero or negative repetitions changes their observable `0` or negative receiver return. The value can escape through method, block, lambda, conditional, begin/ensure, class, assignment, and argument expressions.

Even where the value appears unused, removing a block can change lexical bindings and its runtime frame. For example, a local first assigned within the block must not become an outer local. Keeping the original source avoids assumptions about reflection and block-local scope that the former correction guards could not prove. The correction-only renderer, statement proof, and scope machinery have been removed; count parsing and diagnostic matching are unchanged.

`tests/fixtures/times-value-safety.json` records 16 examples refreshed against the project bundle's RuboCop 1.65.1. Explicit reference exceptions preserve the Ruby diagnostics while declaring corrections unavailable. The integration safety test checks unchanged bytes and noncorrectable diagnostics in Off, Safe, and All modes, including formerly positive unused-statement controls. RubyVM compilation and isolated Ruby runtime comparisons passed for all 16 original and retained sources; the first three methods still return `1`, `0`, and `-1`. A `2.times` control remains offense-free.
