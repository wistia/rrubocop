# Repeated file processing: integrated follow-up

Code checkpoint `8dc381b` combines layout/association, config startup/exclusions and shared regexp/heredoc improvements. All assigned findings have implemented fixes or an explicit false-positive disposition: SpecFilePathFormat has only one eligible top group per file, so its proposed repeated intra-file regex compilation was not real.

The same 14,824-file manifest takes **88.309s uncached**, versus **89.408s** in the preceding optimization pass (1.23% lower wall time). User CPU is683.166s, system CPU5.964s, peak RSS425,512KB. Warm timing is **0.457s**. Entire parsed uncached JSON matches the original validated baseline: the same three offenses and metadata, with empty stderr. Exit1 is expected for those offenses. Single separately measured cold runs do not establish that this small wall-time difference exceeds normal variation. The lint result cache was disabled; OS caches were not flushed.

Combined tests: **1,129 passed, zero failed, one ignored**,110.68s. Department evidence includes2,754 layout/association before-after mode results and3,531 shared-helper results, all byte-identical, plus26 runtime checks. Startup tests verify scoped lockfile snapshots, negative-read lifetime, preserved per-plugin fallback/warnings and exact global exclusion matching. Correction syntax guards remain intact. There are no release uploads or rollout changes.

Complete benchmark invocations, raw JSON, stderr and status are retained at `/tmp/rrubocop-efficiency-full-8dc381b`; committed measurement/hashes are in `docs/evidence/repeat-processing-full-corpus.json`.
