pub(super) fn for_rule(name: &str) -> &'static [&'static [u8]] {
    CALL_NAMES
        .iter()
        .find_map(|(rule, names)| (*rule == name).then_some(*names))
        .unwrap_or(&[])
}

const CALL_NAMES: &[(&str, &[&[u8]])] = &[
    ("AnyCable/PeriodicalTimers", &[b"periodically"]),
    ("AnyCable/StreamFrom", &[b"stream_from", b"stream_for"]),
    ("Cops/CreateTableMustIncludeCharset", &[b"create_table"]),
    ("Cops/CreateTableMustIncludeCollation", &[b"create_table"]),
    ("Cops/NoCSVGenerate", &[b"generate"]),
    (
        "Cops/NoCallbackSkippingProtectedWrites",
        &[b"update_column", b"update_columns", b"update_all"],
    ),
    (
        "Cops/NoDirectLanternConfigBuilderMutators",
        LANTERN_MUTATORS,
    ),
    ("Cops/NoForeignKeys", &[b"add_foreign_key"]),
    ("Cops/NoRequireOnAutoLoadedPaths", &[b"require"]),
    ("Lint/DuplicateRequire", &[b"require", b"require_relative"]),
    ("Cops/NoSetWithoutExpiration", &[b"set"]),
    ("Cops/NoSetex", &[b"setex"]),
    ("Cops/NoSidekiqReceive", &[b"receive"]),
    ("Cops/NoSleep", &[b"sleep"]),
    ("Custom/RequireRespondToAny", &[b"respond_to"]),
    ("Lint/ErbNewArguments", &[b"new"]),
    ("Lint/FloatComparison", &[b"==", b"!=", b"eql?", b"equal?"]),
    ("Lint/IdentityComparison", &[b"=="]),
    ("Lint/RaiseException", &[b"raise", b"fail"]),
    (
        "Lint/RedundantWithIndex",
        &[b"each_with_index", b"with_index"],
    ),
    (
        "Lint/RedundantWithObject",
        &[b"each_with_object", b"with_object"],
    ),
    ("Lint/RefinementImportMethods", &[b"include", b"prepend"]),
    (
        "Lint/TrailingCommaInAttributeDeclaration",
        &[b"attr", b"attr_reader", b"attr_writer", b"attr_accessor"],
    ),
    (
        "Lint/UriEscapeUnescape",
        &[b"escape", b"encode", b"unescape", b"decode"],
    ),
    ("Lint/UriRegexp", &[b"regexp"]),
    ("Lint/UselessTimes", &[b"times"]),
    ("Security/CompoundHash", &[b"hash", b"^", b"+", b"*", b"|"]),
    ("Serializers/AvoidCapitalizedKeys", &[b"attribute"]),
    ("Serializers/AvoidKeyAndBlock", &[b"attribute"]),
    ("Serializers/AvoidMethodAndBlock", &[b"attribute"]),
    (
        "Serializers/PreferAttributeBlockOverAttributeAndMethod",
        &[b"attribute"],
    ),
    (
        "Serializers/PreferAttributeBlockOverAttributesAndMethods",
        &[b"attributes"],
    ),
    (
        "Style/ClassEqualityComparison",
        &[b"==", b"eql?", b"equal?"],
    ),
    ("Style/Dir", &[b"expand_path", b"dirname"]),
    (
        "Style/EvalWithLocation",
        &[b"eval", b"class_eval", b"module_eval", b"instance_eval"],
    ),
    ("Style/FileRead", &[b"open"]),
    ("Style/FileWrite", &[b"open"]),
    ("Style/HashConversion", &[b"[]"]),
    ("Style/HashExcept", &[b"reject", b"select", b"filter"]),
    (
        "Style/MapCompactWithConditionalBlock",
        &[b"compact", b"filter_map"],
    ),
    ("Style/MixinUsage", &[b"include", b"extend", b"prepend"]),
    ("Style/SlicingWithRange", &[b"[]"]),
    (
        "Style/RandomWithOffset",
        &[b"succ", b"pred", b"next", b"+", b"-"],
    ),
    (
        "Style/RedundantFileExtensionInRequire",
        &[b"require", b"require_relative"],
    ),
    (
        "Style/RedundantSort",
        &[b"first", b"last", b"at", b"slice", b"[]"],
    ),
    (
        "Style/UnpackFirst",
        &[b"first", b"last", b"at", b"slice", b"[]"],
    ),
    ("Style/StderrPuts", &[b"puts"]),
    ("Style/StringChars", &[b"split"]),
    ("Style/TopLevelMethodDefinition", &[b"define_method"]),
];

const LANTERN_MUTATORS: &[&[u8]] = &[
    b"append_element",
    b"update_element",
    b"remove_element",
    b"remove_elements",
    b"replace_element_with",
    b"move_element",
    b"copy_element",
    b"set_music",
    b"clear_music",
    b"set_narration",
    b"clear_narration",
    b"set_target_duration",
    b"clear_target_duration",
    b"set_theme_tokens",
    b"clear_theme_tokens",
    b"set_aspect_ratio",
    b"update_timing",
    b"remove_timing",
    b"update_marker",
    b"remove_marker",
    b"set_timings",
    b"set_markers",
];
