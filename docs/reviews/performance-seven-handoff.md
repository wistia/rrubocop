# Seven Performance rule handoff

Branch: `feat/wistia-parity-performance` on the `wistia` fork, starting at
framework checkpoint b4ffbb8. The completed RSpec branch remains untouched at
0c55d2c. All seven transferred classes now implement their original enabled rule.
See performance-seven-parity.md/json for per-rule counts, source paths and every
retained discrepancy. The isolated-rule strict comparison still exits one.

Final evidence: 5,028 originals plus 10,056 generated states, 23 trusted programs,
zero diagnostic/metadata/invalid-generated-syntax/idempotence differences on valid
original sources; all 46 native trusted runtime states preserve stdout/exit status.
There are 4,134 safe-byte, 4,485 all-byte and 27 runtime differences, plus six
original_invalid states from two imported unscoped yield hosts. No discrepancy
allowlist is present. Shared non-UTF8 and external metaprogramming families remain
bounded. Generic incompatible-cop scheduling is explicitly deferred; RegexpMatch
names ConstantRegexp as its original incompatible cop in the report.

Validation: 931 native library tests pass, one ignored; 103 actual production CLI
scope controls across 47 rules pass, including seven positives/excluded paths.
Ruby -A with the actual Wistia config leaves one inherited RescueMustReportError
finding in the standalone JSON-reporting reference harness. New Ruby scripts are
clean. Rust formatting and non-test binary build pass.

## Shared integration points

- The base is the framework branch, so merge only the final owned changes when
  integrating onto a newer framework/core checkpoint. Do not overwrite newer
  unrelated model implementations with the older prerequisite snapshot bf90208.
- 79c8aab imports original shorthand AST roles (framework d0a2f14). e7f2cc8 adds
  shared ruby_semantics::pure/value_used and delegates DoubleStartEndWith. It has
  101 direct original AST observations; the existing DoubleStart gate retains its
  explicit earlier safeguards/original errors, with no new purity-case mismatch.
- 832b08e exposes regular_send_name/regular_send_arguments and tests shorthand
  projections. The original 48-role golden is preserved.
- bef4c25 imports numbered_bindings and cached binding identities (framework
  8be32c7). 4776f8e removes the duplicate identical binding test from that import.
  Retain the byte-to-introduction identity map, numbered block metadata and Core's
  synthetic-read filter when combining model/context changes.
- fd52bae adds fixture GlobalConfig and Detect's production sibling preferred-name
  injection. Keep existing GemVersions handling. The actual Wistia default is find;
  original explicit nil falls back to detect.
- The final Regexp checkpoint exposes existing regexp capture counts through a
  thin ported wrapper and static regex text/options through the shared decoder.
  Shared regexp helpers now guard English aliases and explicit reflective
  last_match observers. No second literal decoder or numbered-variable scanner
  was introduced.
- Keep the seven-cop fixture modules separate and union their hooks into the
  framework cases/runtime/scope harnesses. Preserve the newer framework fixture
  union. The report files here intentionally use separate filenames.

## Reproduction

Use actual original Ruby in /home/ubuntu/code/wistia-rrubocop-pr; bundle exec
rubocop resolves to Rust. The harness invokes the original Ruby classes directly
and scope controls invoke bin/rubocop-ruby. Build on tmpfs with DEBUG=0 and
INCREMENTAL=0. The strict seven-rule gate is:

```sh
env CARGO_TARGET_DIR=/dev/shm/rrubocop-rspec-target CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 PATH=/home/ubuntu/.cargo/bin:$PATH ASDF_RUBY_VERSION=3.4.9 python3 -c 'import sys,runpy;sys.path.insert(0,"scripts/framework_parity");import cases;from performance_seven_cases import COPS;original=cases.cases;cases.cases=lambda:[r for r in original() if r["cop"] in COPS];runpy.run_path("scripts/framework_parity/check.py",run_name="__main__")' --ruby-project /home/ubuntu/code/wistia-rrubocop-pr --output /dev/shm/performance-seven-replay
```

The committed update_performance_report.py merges strict result directories in
oldest-to-newest order and explicitly excludes generated IDs from original counts.
Final artifacts remain in /dev/shm/performance-seven-two-rules,
performance-three-latest, performance-detect-string-final, performance-regexp-first
and performance-regexp-edges-final. Additional native/scope/lint logs use the
performance-seven-final prefix. The upstream importer statically expands 4,720
source/config specimens without executing them. Runtime execution is limited to
explicitly authored programs in performance_seven_runtime.py.

No release, S3 publication, rollout branch or Wistia PR was modified.
