# Registered plugin classes without YAML entries

The initial 553-rule inventory includes GraphQL/PrepareMethod. The pinned
rubocop-graphql 1.5.4 registers this class but omits it from config/default.yml.
Original Config#for_cop gives the registered class Enabled:true; native previously
inferred version availability solely from YAML names and incorrectly disabled it.

The exact-version registry supplement records this upstream registration without
editing upstream YAML or user config. Embedded plugin loading merges its default
Enabled:true only when no YAML entry exists. Subsequent user disablement and
unloaded-plugin/version gates still apply. No future plugin versions are inferred.
Disk-provided custom plugin configs remain outside this vendored metadata fix.

The original observation in tests/fixtures/plugin-registration-original.json
records the installed pinned gem version, class source path and SHA256 of both
class source and original default YAML. It was obtained by requiring rubocop and
rubocop-graphql, selecting GraphQL classes in Registry.global absent from the raw
YAML keys, and inspecting on_send.source_location. The only missing class was
GraphQL/PrepareMethod. This fixes configuration availability; the existing rule's
full behavioral parity review remains in the core review queue.

The updated full native suite passes 1,085 tests with one ignored. The
check_plugin_registration.py runner compares three actual Ruby/native CLI
controls: positive availability, user disablement without --only, and plugin
file-scope exclusion. All three pass. Disablement deliberately omits --only
because Ruby's --only explicitly forces a disabled cop; that separate CLI
selection contract is outside this availability fix.
