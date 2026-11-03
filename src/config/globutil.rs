//! Glob / Ruby-regexp helpers for Include/Exclude patterns.

use std::path::Path;

use globset::{GlobBuilder, GlobMatcher, GlobSet, GlobSetBuilder};
use regex::RegexSet;

/// Check if a pattern string is a Ruby regexp (from `!ruby/regexp /pattern/`).
pub(crate) fn extract_ruby_regexp(s: &str) -> Option<&str> {
    let s = s.trim();
    if !s.starts_with('/') || s.len() <= 1 {
        return None;
    }
    let end = s[1..].rfind('/')?;
    let after_close = &s[end + 2..];
    if !after_close
        .bytes()
        .all(|b| matches!(b, b'i' | b'x' | b'm' | b'u' | b's' | b'n' | b'o' | b'e'))
    {
        return None;
    }
    Some(&s[1..end + 1])
}

fn expand_one_star_star(
    current: &str,
    expanded: &mut std::collections::HashSet<String>,
    pending: &mut Vec<String>,
) {
    let mut start = 0;
    while let Some(idx) = current[start..].find("**/") {
        let idx = start + idx;
        let mut variant = current.to_string();
        variant.replace_range(idx..idx + 3, "");
        if expanded.insert(variant.clone()) {
            pending.push(variant);
        }
        start = idx + 1;
    }
}

/// Expand zero-depth `**/` variants (RuboCop-compatible).
pub(crate) fn expand_zero_depth_globs(pattern: &str) -> Vec<String> {
    let mut expanded = std::collections::HashSet::from([pattern.to_string()]);
    let mut pending = vec![pattern.to_string()];
    while let Some(current) = pending.pop() {
        expand_one_star_star(&current, &mut expanded, &mut pending);
    }
    let mut variants: Vec<_> = expanded.into_iter().collect();
    variants.sort();
    variants
}

fn try_add_glob(builder: &mut GlobSetBuilder, pattern: &str) -> bool {
    match GlobBuilder::new(pattern).literal_separator(true).build() {
        Ok(glob) => {
            builder.add(glob);
            true
        }
        Err(_) => false,
    }
}

/// Build a `GlobSet` from pattern strings, skipping Ruby regexp patterns.
pub(crate) fn build_glob_set(patterns: &[&str]) -> Option<GlobSet> {
    if patterns.is_empty() {
        return None;
    }
    let mut builder = GlobSetBuilder::new();
    let mut count = 0;
    for pat in patterns {
        if extract_ruby_regexp(pat).is_some() {
            continue;
        }
        for expanded in expand_zero_depth_globs(pat) {
            if try_add_glob(&mut builder, &expanded) {
                count += 1;
            }
        }
    }
    if count == 0 {
        return None;
    }
    builder.build().ok()
}

/// Build a `RegexSet` from Ruby regexp patterns in the list.
pub(crate) fn build_regex_set(patterns: &[&str]) -> Option<RegexSet> {
    let regexes: Vec<&str> = patterns
        .iter()
        .filter_map(|p| extract_ruby_regexp(p))
        .collect();
    if regexes.is_empty() {
        return None;
    }
    RegexSet::new(&regexes).ok()
}

pub(crate) fn build_exact_glob_matchers(patterns: &[&str]) -> Vec<GlobMatcher> {
    patterns
        .iter()
        .filter(|pattern| extract_ruby_regexp(pattern).is_none())
        .flat_map(|pattern| expand_zero_depth_globs(pattern))
        .filter_map(|pattern| {
            GlobBuilder::new(&pattern)
                .literal_separator(false)
                .build()
                .ok()
        })
        .map(|glob| glob.compile_matcher())
        .collect()
}

pub(crate) fn exact_glob_matches(matcher: &GlobMatcher, path: &Path) -> bool {
    matcher.is_match(path) || matcher.is_match(path.to_string_lossy().as_ref())
}

fn glob_expanded_matches(expanded: &str, path: &Path) -> bool {
    let Ok(glob) = GlobBuilder::new(expanded).literal_separator(false).build() else {
        return false;
    };
    let matcher = glob.compile_matcher();
    exact_glob_matches(&matcher, path)
}

/// Match a RuboCop-style glob (or Ruby regexp) against a file path.
pub(crate) fn glob_matches(pattern: &str, path: &Path) -> bool {
    if let Some(re_pattern) = extract_ruby_regexp(pattern) {
        return regex::Regex::new(re_pattern)
            .map(|re| re.is_match(&path.to_string_lossy()))
            .unwrap_or(false);
    }
    expand_zero_depth_globs(pattern)
        .iter()
        .any(|expanded| glob_expanded_matches(expanded, path))
}
