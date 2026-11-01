# Retained autocorrection guard fixes

Review baseline: `789661a38997290c938c4920e06a92585888ed63`. Implementation: `342454a354dc4fb981d8745c9b8ca014f5cb4cc2`.

## Resulting behavior

MixinGrouping is diagnostic-only in both styles, including with `-A`. Inclusion hooks can create or replace constants, so even contiguous constant-only rewrites lack a reliable safety proof. Diagnostics, messages, configuration styles, and locations remain supported. Its previous renderer and correction-safety machinery are removed. [Scope and hook policy details](retained-scope-hook-safety.md).

UselessTimes declines parameter substitution through method/class/module/singleton scopes or matching pattern-binding contexts. Ordinary captured nested lambdas and unrelated method definitions remain covered by positive controls. This prevents unrelated method calls becoming `0` and pattern captures becoming literal patterns.

The remaining fixes cover eight rules:

| Rule | Guard or correction |
| --- | --- |
| RedundantConditional | Require single-statement arms and Ruby's supported boolean-comparison operator set; reject integer-returning match/spaceship operations. Preserve comments by leaving rewrites manual. |
| OrAssignment | Distinguish invalid multi-statement arms from absent arms. Require a real local binding for bare identifier conditions, preserving method calls before later assignments. |
| HashExcept | Require exactly one block expression; extra statements do not match. Comment-bearing rewrites remain manual. |
| MapCompactWithConditionalBlock | Require singleton branch bodies and compatible opposite arms. Argument-bearing uncertain `next` guards remain manual instead of reproducing the upstream wrong filter direction. |
| KeywordParametersOrder | Reorder only stable literal defaults; dynamic defaults and comments remain manual. |
| TrailingBodyOnMethodDefinition | Keep first-line method comments in place, including magic and cop-directive comments. No comment-hoisting helper remains. |
| FileWrite | Guard removed block parameters, evaluation/truncation order, coercion, comments, heredocs, and unsupported call shapes. Local content is allowed only with a static string path; literal content retains ordinary supported rewrites. |
| RedundantAssignment | Leave assignments observable through closures or binding/eval observers manual. Skip observer traversal when no candidate exists. |

The shared branch matcher now explicitly distinguishes an absent arm from a present multi-statement arm. Independent review caught an initial ambiguity that could otherwise delete an `unless` else-arm. Binding-aware checks reuse the file model when available.

## Reference policy

Several dangerous rewrites reproduce in bundled Ruby RuboCop 1.65.1. Those cases retain explicit source-preserving or no-offense overrides with reasons. Rust-only false positives now follow Ruby. No fixture silently treats a Ruby exception as a clean result.

The existing unsafe-only Times return-value/lexical-scope assumptions remain documented. HashExcept and MapCompact retain their upstream unsafe input/evaluation assumptions; these fixes address discarded statements, invalid inference, and the demonstrated control-flow shapes. They do not claim every `-A` rewrite is behavior-equivalent for arbitrary inputs.

Cache rules revision advances to 33, invalidating earlier correctability results. No application source files were modified.

## Validation

- 786 tests pass across all targets. The final no-candidate observer fast path also passes the two focused four-rule safety tests.
- The port reference corpus now has 2,304 cases plus 30 earlier parity regressions: 2,334 across the 98 ports. New dedicated matrices contain 13 scope/hook, 37 method/keyword/file/assignment, and 50 branch/block cases (100 targeted cases; 99 newly unique central records).
- The release CLI passes 300 comparisons across Off/Safe/All. All 100 original and expected outputs compile with bundled RubyVM. Thirteen scope/hook and ten four-rule runtime comparisons preserve observations; eight additional original-versus-old-unsafe runtime examples explain the branch/block guards. Exact bytes are checked by the engine and release CLI.
- 67 earlier CLI inspection probes and 21 earlier correction probes remain identical. Independent review caught and verified the multi-arm ambiguity and bare-method OrAssignment fixes before integration.
- Formatting, diff checks, and global ABC pass: zero function/module complexity violations, 93 used-once advisories, zero never-used warnings. Clippy has 223 preexisting diagnostics, with no new diagnostic groups or suppression changes.

## Full-codebase comparison

One matched release pair, cache reads disabled and writes isolated by binary, with warm filesystem and no concurrent builds or agent scans from this task:

| Binary | Wall seconds | User CPU seconds | Peak RSS KiB | Files | Findings |
| --- | ---: | ---: | ---: | ---: | ---: |
| before | 30.748 | 152.814 | 135504 | 14762 | 27 |
| after | 30.547 | 153.415 | 134080 | 14762 | 27 |

The complete CLI offense arrays match. This single pair validates scope and output, not a performance improvement claim. Host load differs from earlier approximately 20-second batches; compare within this pair rather than across reports. The underlying work remains similar (about 153 aggregate user-CPU seconds).

[Checks, runtime observations, hashes, raw timings, and runners](retained-autocorrect-guard-measurements.json).
