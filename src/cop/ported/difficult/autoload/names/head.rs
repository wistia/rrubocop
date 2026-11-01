pub(super) fn join(parent: Option<&str>, name: &str, limit: usize) -> String {
    let mut result = String::new();
    if let Some(parent) = parent {
        append(&mut result, parent, limit);
        append(&mut result, "::", limit);
    }
    append(&mut result, name, limit);
    result
}
fn append(result: &mut String, text: &str, limit: usize) {
    if result.len() >= limit {
        return;
    }
    let mut end = text.len().min(limit - result.len());
    while !text.is_char_boundary(end) {
        end += 1;
    }
    result.push_str(&text[..end]);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_comparison_matches_complete_names_at_segment_and_unicode_boundaries() {
        for expected in ["", "A", "A::B", "Object::A", "N", "🌟", "N🌟::Ü"] {
            for actual in [
                "",
                "A",
                "A::",
                "A::B",
                "A::Bee",
                "Object::A",
                "Object::Object::A",
                "Object::N🌟",
                "Object::🌟::B",
                "N🌟::Ü::Tail",
            ] {
                let bounded = join(None, actual, expected.len() + 10);
                let strip = |s: &str| s.strip_prefix("Object::").unwrap_or(s).to_owned();
                assert_eq!(
                    super::super::super::compatible(expected, &strip(actual)),
                    super::super::super::compatible(expected, &strip(&bounded)),
                    "{expected:?} {actual:?} {bounded:?}"
                );
            }
        }
    }
    #[test]
    fn bounds_include_the_complete_character_past_the_limit() {
        assert_eq!(join(Some("Object"), "N🌟", 10), "Object::N🌟");
        assert_eq!(join(Some(""), "B", 10), "::B");
        assert_eq!(join(None, "A::Bee", 4), "A::B");
    }
}
