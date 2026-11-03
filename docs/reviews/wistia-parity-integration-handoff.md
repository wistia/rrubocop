# Integration handoff

Branch: `feat/wistia-parity-integrated` on the wistia fork. The validated code
checkpoint is 362240f; later commits record proof only. The working tree is clean
and the source branches, Wistia rollout PR and S3 artifacts remain unchanged.

The read-only paired run covers exactly 14,824 Wistia files at
310b8e42e91232d07210eff6cacc2f2035a01bda. Ruby and native each report the same
three diagnostics; both inspect all targets and have empty stderr. Complete raw
stdout JSON, stderr, manifest, arguments and statuses are retained at
/tmp/rrubocop-full-paired-362240f. Committed summary and SHA256s are in
docs/evidence/integrated-full-corpus-parity.json. The immutable optimized binary
is /dev/shm/rrubocop-full-paired/rrubocop-362240f, built in the detached source
worktree /dev/shm/rrubocop-full-362240f. No build archives were published.

The full native suite passes 1,096 tests with one ignored. The new 74-file
reproducer also accepts --manifest for repeating the complete comparison:

```sh
python3 scripts/parity_audit/reconcile_real_corpus.py \
  --ruby-project /home/ubuntu/code/wistia-rrubocop-pr \
  --native /dev/shm/rrubocop-full-paired/rrubocop-362240f \
  --manifest /tmp/rrubocop-full-paired-362240f/targets.json \
  --output /dev/shm/rrubocop-full-repeat
```

This repeats a long original Ruby run; the completed run took 1,115.67 seconds.
The optimized native run took 99.75 seconds and its warm repeat 0.47 seconds.
Current-corpus agreement concerns read-only diagnostics. Autocorrection safeguards,
parser/configuration/version limits and user-deferred combined-cop scheduling
remain explicit in the family reports.

Separate queued checkpoints are not part of this proof: selector dispatch
8873ea0; generic send wrappers 7d7538c with SkipsModelValidations cbaed47;
DepartmentName 0a835af; FindEach 1a682eb; and TimeZone detection-only 456d27d,
whose correction review is unfinished. Future integration should freeze each
coherent tested batch and retain this verified snapshot for comparison.
