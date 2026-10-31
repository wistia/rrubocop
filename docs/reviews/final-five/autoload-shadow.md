Independent read-only review of 832a13f

P2 — Lint/ShadowedArgument misses nested/rest destructured arguments.
Location: src/model/scopes.rs:85 (declare_wrapped_param).
Minimal reproducer: `def m((a, (b, c))); b = 42; puts b; end`.
Existing actual CLI evidence in /tmp/rr-five-review-scope.json: Rust emits zero offenses, pinned Ruby emits one on `b = 42`. Same discrepancy for block `items.each do |(a, (b,c))|; b = 42; puts b; end` and method `def m((a, *rest)); rest = []; puts rest; end`.
Root cause confirmed by inspection: only immediate identifier children are bound; deeper destructuring/splat wrapper children are skipped. The shadow port already accepts destructured/splat ancestors in parameter_wrapper, so the missing model declaration prevents the check from considering these parameters. Recommend recursively traversing parameter wrappers (avoid default expressions) and adding all three cases to both reference fixtures and real pipeline tests.

P2 — Lint/ShadowedArgument reports pattern rebinding that Ruby does not flag.
Location: src/cop/ported/difficult/shadow.rs:97, with src/model/patterns.rs:114.
Minimal reproducer: `def m(foo); data => { foo: }; puts foo; end`.
Existing CLI evidence: Rust emits one offense on pattern `foo:`, pinned Ruby zero. Pattern binding is recorded as WriteKind::Masgn, retaining the existing argument introduction; candidate accepts this as an ordinary shadow write. Ruby treats this as match_var rather than the assignment callbacks used by this cop. Recommend retaining pattern writes for model consumers, but excluding pattern binding nodes from this cop's candidates. Add a pattern rebind regression and a later ordinary assignment control.

Coverage: inspected 142 current ShadowedArgument reference cases: none cover these nested/rest destructuring or pattern rebinding reproducers. Reused existing eight-case CLI artifact rather than duplicating its execution.

Autoload review: no additional confirmed material mismatch found. Eleven new isolated-project comparisons passed using the existing release binary and pinned Ruby Commissioner, with identical actual root directory ordering. Cases cover supported literal acronym inflections API/OAuth/HTML, acronym-plus-digit constants, explicit absolute nested class definitions, nested class_eval, Class.new assignment namespace, multiple constant assignment, and ||= constant assignment. Probe artifact: /tmp/rr-five-final-autoload-probes.json. The root/namespace checks supplement the existing 44 cases. Acronym loading statically extracts literal calls rather than executing arbitrary initializer code; dynamic/configuration-dependent inflection behavior remains a limitation, not a demonstrated discrepancy for the supported literal initializer paths examined.

Both cops are diagnostic-only. The previous CLI -a/-A no-edit checks in /tmp/rr-five-cli.json already establish that explicit AutoCorrect/SafeAutoCorrect overrides still do not edit files. These findings affect diagnostic parity, not autocorrection safety.

No repository modifications, cargo builds, full test runs or full-codebase scans performed.
