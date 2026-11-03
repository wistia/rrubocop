# Original sibling configuration fixtures

The framework differential harness accepts an explicit GlobalConfig dictionary in
fixture configuration. Original Ruby merges it into the loaded production Config;
the native audit inserts the named sibling CopConfig entries before normal
resolution. Existing GemVersions handling is preserved. This bridge changes the
audit only; production Detect receives its preferred name through ordinary sibling
option injection from Style/CollectionMethods.PreferredMethods.detect.

The loaded Wistia configuration prefers find by default. The original upstream
Detect specs override the sibling map and test nil, detect and find. A missing or
nil explicit preferred entry falls back to detect in the original implementation.
These branches are represented separately instead of silently treating all spec
sources as production-default configuration. Invalid configuration coercions are
not considered general support merely because a fixture is statically extractable.

The importer never executes upstream examples: Prism decodes scalar literals,
lexical let values, RuboCop::Config.new with a literal dictionary, shared-example
arguments and closed literal loops. It records the resulting source/config
specimens. Only the separately authored trusted runtime module executes programs.
