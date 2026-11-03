# Exact framework optimization proof

These gzip files use deterministic headers (`mtime=0`). `manifest.json` records
compressed and decompressed hashes, complete source revisions and comparison
counts. `native-both.jsonl.gz` preserves both baseline and optimized output:
their original raw files were byte-identical, with the same SHA256, so storing
one shared copy loses no observations.

The inputs contain 6,383 initial specimens and 12,766 derived final source/config
states. The unchanged `custom_parity_audit_probe` runs each in off/safe/all modes,
returning full diagnostics (including correctability and correction flags) and
corrected source. It parses/lints fixture data. The dedicated audit executable
also supports the in-memory schema fixtures that are not exposed by a production
CLI, which is why source-equivalent baseline test builds were used for this
complete comparison, in addition to the exact 74-file public CLI control.

To reproduce, decompress `inputs.json.gz`, check out the manifest's baseline or
optimized source, and run the existing audit probe with
`RRUBOCOP_AUDIT_INPUT` pointing to that JSON file and `RRUBOCOP_AUDIT_OUTPUT`
pointing to a new JSONL file:

```
cargo test --locked --lib custom_parity_audit_probe -- --ignored
```

Both runs used the same working directory; no input path contains `$PROJECT`,
so manifest-directory substitution cannot explain their equality. Compare the
complete result file against the decompressed `native-both.jsonl.gz` hash.

`ruby-reference.jsonl.gz`, `strict-ruby-failures.json.gz`, and
`trusted-runtime.json.gz` preserve the strict Ruby comparison as reported,
including all 11,100 differences. They demonstrate inherited parity/safety
limits; they are not an exception list and the strict checker still exits
nonzero. No original fixture or expected result was changed for this proof.
