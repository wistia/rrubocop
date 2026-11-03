mod anchored_match;
mod anchored_regexp;
mod anchored_substitution;
mod big_decimal_with_numeric_argument;
mod bind_call;
mod caller;
mod compare_with_block;
mod count;
mod delete_prefix;
mod delete_suffix;
mod detect;
mod double_start_end_with;
mod end_with;
mod fixed_size;
mod flat_map;
mod inefficient_hash_search;
mod range_include;
mod redundant_block_call;
mod redundant_match;
mod redundant_merge;
mod regexp_match;
mod reverse_each;
mod send_range;
mod size;
mod start_with;
mod string_replacement;
mod times_map;
mod unfreeze_string;
mod uri_default_parser;

use crate::cop::registry::CopRegistry;

pub fn register_all(registry: &mut CopRegistry) {
    crate::register_cops!(registry;
        big_decimal_with_numeric_argument::BigDecimalWithNumericArgument,
        bind_call::BindCall,
        caller::Caller,
        compare_with_block::CompareWithBlock,
        count::Count,
        delete_prefix::DeletePrefix,
        delete_suffix::DeleteSuffix,
        detect::Detect,
        double_start_end_with::DoubleStartEndWith,
        end_with::EndWith,
        fixed_size::FixedSize,
        flat_map::FlatMap,
        inefficient_hash_search::InefficientHashSearch,
        range_include::RangeInclude,
        redundant_block_call::RedundantBlockCall,
        redundant_match::RedundantMatch,
        redundant_merge::RedundantMerge,
        regexp_match::RegexpMatch,
        reverse_each::ReverseEach,
        size::Size,
        start_with::StartWith,
        string_replacement::StringReplacement,
        times_map::TimesMap,
        unfreeze_string::UnfreezeString,
        uri_default_parser::UriDefaultParser,
    );
}
