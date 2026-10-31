# Application parity experiment

Branch: `fix/wistia-rubocop-parity`, based on rrubocop v0.8.17 (`0b670724090f2872b2df25b26bf1fad955d2d253`).

The acceptance corpus is a Rails application pinned to RuboCop 1.65.1, Standard 1.40.0, rubocop-rails 2.26.0, rubocop-rspec 3.0.4, rubocop-graphql 1.5.4, rubocop-performance 1.21.1, and rubocop-factory_bot 2.26.1. Its unmodified Ruby RuboCop run inspected 14,763 files with zero offenses.

The original rrubocop release reported 3,649 offenses and inspected 14,674 files. Explicit Ruby 3.4 reduced the original result to 3,260 offenses across 54 cops. This branch fixes the observed differences in the linter rather than reformatting the application or turning off its configured cops.

## Changes

- Vendor exact gem configurations for the pinned versions rather than silently selecting nearby releases.
- Infer Ruby from `.tool-versions` after explicit configuration, gemspec constraints, and `.ruby-version`.
- Carry selected core/Rails cop versions into effective cop options for tested legacy behaviors, including conservative argument forwarding, hash-pattern spacing, and timezone-string recognition.
- Respect fixed argument indentation when hash alignment would conflict with it. Correct array anchors and prefix/postfix keyword indentation.
- Distinguish multiline hash literals from pattern matches, comments, heredocs, and interpolation syntax.
- Fix inline exception-layout checks, legacy `Department::Cop` directives, and chained todo directives.
- Restrict method-name checks to the relevant receivers; preserve explicit receivers required for keywords, Kernel methods, safe navigation, and multiple-assignment targets.
- Improve scope-sensitive checks for private singleton methods, GraphQL DSL constructs, format argument counts, and captured variable writes.
- Require a terminal else before considering conditional branches identical.
- Match legacy plugin-exclusion replacement, discover recognized Ruby dotfiles, and follow directory symlinks without scanning hidden directories.
- Bump the lint cache revision to invalidate prior diagnostics.

## Results

Final local ARM64 run (8 Neoverse-V1 cores, Ruby 3.4.9):

| Run | Inspected files | Offenses | Wall time |
| --- | ---: | ---: | ---: |
| Bundled Ruby RuboCop, fresh cache | 14,763 | 0 | 1,139.79s |
| Original rrubocop 0.8.17, fresh cache | 14,674 | 3,649 | 21.78s |
| This branch, cache reads disabled | 14,763 | 0 | 24.80s |

The final file set equals the bundled RuboCop file set (no missing or extra paths). Peak RSS was 145,492 KiB. These are local single observations, not CI predictions.

`cargo test --locked`: 604 tests pass, plus binary/doc-test targets. `abcop --full` (0.23.0): zero ABC, module-ABC, used-once, or never-used findings. There are 45 focused cop fixtures, 19 positive-control snippets, and config/discovery/directive regression tests. Of the cop fixtures, 44 were separately checked against the application's bundled Ruby cops and all were clean; the remaining directive-range case is tested directly in Rust.

## Validation and limits

`src/testutil/project_parity.rs` exercises focused generic regression fixtures under `tests/fixtures/cops/`. Positive controls ensure representative cops still detect bad code. Config, file-discovery, and directive behavior have separate tests. The application itself and its lint configuration are unchanged.

The Ruby reference comparison uses the application's **bundled** RuboCop and plugin versions. Findings are compared with caches disabled; file lists are compared as sets, including paths through directory symlinks. Tests also preserve the existing newer-version fixtures.

This is diagnostic parity on a clean corpus, not proof of complete cop implementation or autocorrection parity. The original inventory still has 111 application-enabled cops absent from the registry, including custom Ruby cops. Those missing implementations are not supplied by this branch. Version boundaries outside the tested pinned versions need a broader compatibility matrix before an upstream release.

## Reproduce

```sh
cargo test --locked
abcop --full
cargo build --release --locked
```

From the application checkout, run the newly built binary with the existing configuration:

```sh
/path/to/rrubocop/target/release/rrubocop --cache false --format json > rrubocop.json
/path/to/rrubocop/target/release/rrubocop -L > rrubocop-files.txt
bundle exec rubocop --no-server --cache false --format json > rubocop.json
```

Use matching Ruby, dependencies, configuration, and input trees. Do not interpret a zero-offense clean-corpus comparison as evidence that unsupported cops have been implemented.
