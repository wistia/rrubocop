# ExactRegexpMatch real-corpus alternation regression

Wistia 310b8e42e91, cops/explicit_sidekiq_queue.rb:32, uses
`const_name&.match?(/\ABaseWorker\z|::BaseWorker\z/)`. Original Ruby RuboCop
1.65.1 reports no offense. Integrated 091dc33 incorrectly diagnosed this as
an exact string match, although its existing tail safeguard declined correction.

Original source: rubocop-1.65.1/lib/rubocop/cop/style/exact_regexp_match.rb,
`exact_regexp_match?`. The original inspects Regexp::Parser root expressions;
an alternation is a root alternation expression, not the required bos/literal/eos
sequence. The native prefix-only predicate had lost this distinction.

The native predicate now checks root alternation, tracking escapes, character
classes, group nesting, inline extended-mode flags and comment text. Eleven fresh
original observations cover the actual ordinary/safe-navigation pattern, root
alternatives, alternatives after groups, nested alternatives, escaped/class pipes,
comment-group pipes and extended-mode comments. Original correction bytes remain
recorded separately from the existing native tail safety guards: original warns
and unsafely removes some suffixes; native retains warning and source bytes.
This patch does not relax those safeguards or execute fixture regexps.

All three scoped tests pass (the 155-state original corpus, existing safeguard
corpus and 11 new observations). A fresh actual Wistia configuration CLI probe on
all 43 files containing the 122 numbered-parameter findings and this finding
reports 0 offenses, exit 0 and empty stderr. Further unusual regexp lexical
families remain subject to the existing bounded predicate review; this is not a
claim of a general Regexp::Parser implementation.
