# Default ActiveSupport inflection properties

`shared::ruby_inflection::{underscore, foreign_key}` follows ActiveSupport 8.1.3.1 `active_support/inflector/methods.rb` with its default empty acronym registry. The Ruby refresh script explicitly checks that registry before recording observations; customized application acronyms are outside this API's scope.

The committed 285-row original fixture checks both functions (570 assertions): root/nested namespace demodulization, acronym/digit/case boundaries, hyphens, every ASCII byte alone and embedded in a name, empty/punctuated names, Unicode capitals/digits and Greek sigma. Ruby's initial ASCII-capital/hyphen/namespace check is preserved. Character-wise Unicode downcasing avoids Rust's contextual final-sigma behavior, which differs from the pinned Ruby default.

Validation: the original-property native test passes; regenerating the fixture is byte-identical. The generator passes pinned RuboCop 1.65.1 using its default configuration. This is a standalone script in the Rust fork, outside Wistia's autoload/spec paths; the Wistia application configuration's custom spec/autoload cops are not claimed as applicable script policy.

This helper is a prerequisite for framework rules; it does not count as replacing another baseline stub.
