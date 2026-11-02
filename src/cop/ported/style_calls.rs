use crate::cop::CopRegistry;
pub(super) fn register(r: &mut CopRegistry) {
    super::rules!(r;
        ("Style/Dir", &["call"], dir, true, true, false),
        ("Style/StringChars", &["call"], string_chars, true, true, false),
        ("Style/StderrPuts", &["call"], stderr, true, true, false),
        ("Style/RedundantFileExtensionInRequire", &["call"], require_extension, true, true, false),
        ("Style/UnpackFirst", &["call","element_reference"], unpack, true, true, false),
        ("Style/RedundantSort", &["call","element_reference"], sort, true, true, false),
        ("Style/SlicingWithRange", &["element_reference","call"], slicing, true, true, false),
        ("Style/RandomWithOffset", &["binary","call"], random, true, true, false)
    );
}

mod basic;
use basic::{dir, require_extension, stderr, string_chars};

mod access;
use access::{sort, unpack};

mod slicing;
use slicing::slicing;

mod random;
use random::random;

mod integer;
