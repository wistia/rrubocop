use crate::cop::CopRegistry;
pub(super) fn register(r: &mut CopRegistry) {
    super::rules!(r;
        ("Style/FileRead", &["call"], file_read, true, true, false),
        ("Style/FileWrite", &["call"], file_write, true, true, false),
        ("Style/HashConversion", &["element_reference","call"], hash_conversion, true, false, false),
        ("Style/HashExcept", &["call"], hash_except, true, true, false),
        ("Style/MapCompactWithConditionalBlock", &["call"], map_compact, false, true, false),
        ("Style/TrailingCommaInBlockArgs", &["block","do_block"], block_comma, true, true, false)
    );
}

mod file;
use file::{file_read, file_write};

mod hash_conversion;
use hash_conversion::hash_conversion;

mod hash_except;
use hash_except::hash_except;

mod map_compact;
use map_compact::map_compact;

mod block_comma;
use block_comma::block_comma;

#[cfg(test)]
mod eligibility_tests;
