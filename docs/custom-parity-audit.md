# Custom cop parity audit

Run the audit from the rrubocop checkout, using the Ruby project whose bundle and custom cops define the baseline:

```sh
python3 scripts/custom_parity_audit.py \
  --ruby-project /home/ubuntu/code/wistia-rrubocop-benchmark \
  --output /tmp/rrubocop-custom-audit
```

Requirements: Python 3, Cargo on PATH, and a working `bundle exec ruby` in the selected Ruby project. The runner does not build a release binary. It builds/runs an ignored Rust test probe through the actual lint/autocorrection pipeline. Normal `cargo test` leaves that probe ignored.

The default run selects all 98 added cops, including the app-specific rules. It combines the central pinned references, additional independent regressions, directive controls, and explicit runtime scenarios. Duplicate examples are removed. The default mutation budget takes two eligible seed examples per cop and generates comment/parenthesis variants plus parser-based percent, encoded, character, and opposite-quote literal forms. AST mutations also cover comments throughout call arguments, explicit operator calls, accessor arguments, attached receiver blocks, numeric spellings, nested literal equivalents, and UTF-8/BINARY source encodings. Mutations remain bounded by the seed budget and do not form a Cartesian product or acquire runtime authorization. Known per-example correction safeguards are not used as ordinary mutation seeds.

The Ruby worker loads the selected project's actual configuration and custom cop definitions. Each cop is isolated and explicitly enabled with the example's recorded options. Correction defaults to enabled in both engines when the example omits `AutoCorrect`; explicit disabled settings are preserved. This avoids unrelated cops mutating the shared AST, and makes comparison differences reproducible. It is not a replacement for a configured full-project CLI scan or a cross-cop interaction suite.

## Checks

- Before correction: offense count, location, message, severity, correctability, and highlight width. Duplicate locations retain their full range multisets.
- Safe and unsafe correction: final rewritten bytes, bounded correction replay, and RubyVM compilation. Native corrections are compiled by Ruby too, catching invalid regexps that the native parser may accept.
- Runtime: the explicit scenarios in `scripts/parity_audit/runtime_cases.json` compare original, native-corrected, and Ruby-corrected behavior in both correction modes. They capture stdout/stderr at the descriptor level, exception class/message, explicit exit status, and watched file bytes.
- Runtime examples get separate temporary directories. Only explicit trusted scenarios execute; reference and generated examples are parsed/compiled, never automatically executed. This is file/process isolation, not an OS security boundary for untrusted Ruby.
- Invalid/looping reference corrections may retain native manual correctability when source is unchanged and all other metadata agrees; native errors and unrelated diagnostic differences still fail.
- Existing fixture exceptions require the relevant explicit `rust_*` override and a reason. Report-only policies are checked separately. They cannot excuse unrelated ranges, severity, native crashes, or internal errors.

The report separates fork differences, documented exceptions, upstream-identical hazards, invalid originals, and invalid generated syntax. Original programs that RubyVM cannot compile are recorded rather than claimed as valid correctness examples. Invalid mutations cannot hide native crashes.

## Output and exit codes

`report.md` gives per-cop example/runtime coverage and grouped findings. `results.json` contains the exact differences and example IDs. `cases.json` contains sources, paths, options, and provenance. `native.jsonl`, `reference.jsonl`, and `runtime.json` retain raw results. `run-meta.json` and `reference-meta.json` record Git state, bundle-lock hash, case-manifest hash, selection, Ruby version, and RuboCop version.

- **0:** no new candidate failures; documented exceptions and inherited hazards do not fail by default.
- **1:** candidate discrepancies found. These are mismatch events, not a count of distinct confirmed bugs; the same example can produce more than one event.
- **2:** incomplete audit, missing dependency, worker failure, or timeout. A partial audit must not be treated as passing.

Reusing an audit output directory replaces its generated reports. Unrelated nonempty directories are rejected. Workers must produce fresh, complete results; stale files cannot stand in for a missing worker.

The runner currently compares full diagnostic metadata before correction. In safe/all modes it compares final source and compilation, not every intermediate diagnostic's remapped location or correction credit. Runtime coverage is deliberately explicit and smaller than the diagnostic corpus; the summary lists uncovered runtime cops. A finite suite cannot prove the absence of all semantic hazards.

## Focused runs

```sh
# No generated mutations; replay all stored examples.
python3 scripts/custom_parity_audit.py --ruby-project /path/to/project \
  --mutations-per-cop 0 --output /tmp/rr-audit-references

# Focus on one rule, retaining its normal mutation budget.
python3 scripts/custom_parity_audit.py --ruby-project /path/to/project \
  --cop Lint/DuplicateRequire --output /tmp/rr-audit-requires

# Reproduce an ID with the same checkout, mutation budget and corpus.
python3 scripts/custom_parity_audit.py --ruby-project /path/to/project \
  --case-id EXAMPLE_ID --output /tmp/rr-audit-reproduction

# Run only the explicit runtime scenarios.
python3 scripts/custom_parity_audit.py --ruby-project /path/to/project \
  --runtime-only --output /tmp/rr-audit-runtime
```

`--limit N` makes a partial run, `--skip-runtime` disables execution, and `--fail-on-upstream-hazards` also fails on inherited hazards. Increase `--mutations-per-cop` to expand the seed budget. Coverage always reflects the selected examples; missing cops are listed.

## Extending the corpus

Add normal pinned examples with their source, cop, path, config, expected diagnostics and correction to the existing reference fixtures. Existing Rust tests continue to validate those independently. The live audit computes fresh Ruby results; it does not refresh or modify the stored reference files.

For behavior checks, add a trusted, self-contained source to `runtime_cases.json`, with `runtime.files` initial contents and `runtime.watch` filenames. Setup/watch paths must be simple filenames. Each scenario's original behavior is its runtime baseline; the code must not contact services, depend on the app, or access external files. Do not attach `runtime` to arbitrary application examples.

Tests for the audit itself:

```sh
AUDIT_RUBY_PROJECT=/path/to/project \
  python3 -m unittest discover -s scripts/parity_audit/tests -v
```

Without `AUDIT_RUBY_PROJECT`, classifier tests still run and Ruby process/descriptor tests are explicitly skipped. Tests inject failures to verify strict exception boundaries, native crashes on invalid mutations, duplicate ranges, incomplete worker output, file-state isolation, binary streams, and exit statuses.

Grouped deep-audit fixtures also exercise forced-enabled autocorrection and reviewed runtime spies; see [the ten-rule deep audit](reviews/deep-custom-rule-audit.md). The separate combined-rule regression suite can be run with `cargo test interacting_custom_corrections_keep_reviewed_output`; it checks reviewed correction output with several cops enabled together. Runtime execution remains restricted to the explicit trusted manifest.

Trusted scenarios default to evaluation in the top-level binding. Set `runtime.execution` to `file` when actual file semantics are required, such as top-level returns; the runner writes and loads `probe.rb` inside its fresh sandbox. Unknown modes are runner failures. This flag is retained only for explicit trusted scenarios and is never inherited by generated mutations.
