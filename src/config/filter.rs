//! Pre-compiled cop Include/Exclude filters.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use globset::GlobSet;
use regex::RegexSet;

use crate::cop::CopRegistry;

use super::globutil::exact_glob_matches;
use super::resolved::ResolvedConfig;

/// Pre-compiled glob filter for a single cop.
pub struct CopFilter {
    pub(crate) enabled: bool,
    pub(crate) include_set: Option<GlobSet>,
    pub(crate) exclude_set: Option<GlobSet>,
    pub(crate) include_re: Option<RegexSet>,
    pub(crate) exclude_re: Option<RegexSet>,
}

impl CopFilter {
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn is_match(&self, path: &Path) -> bool {
        self.enabled && self.is_included(path) && !self.is_excluded(path)
    }

    pub fn is_universal(&self) -> bool {
        self.enabled
            && self.include_set.is_none()
            && self.exclude_set.is_none()
            && self.include_re.is_none()
            && self.exclude_re.is_none()
    }

    pub(crate) fn is_included(&self, path: &Path) -> bool {
        if self.include_set.is_none() && self.include_re.is_none() {
            return true;
        }
        let path_str = path.to_string_lossy();
        if self
            .include_set
            .as_ref()
            .is_some_and(|inc| inc.is_match(path))
        {
            return true;
        }
        self.include_re
            .as_ref()
            .is_some_and(|re| re.is_match(path_str.as_ref()))
    }

    pub(crate) fn is_excluded(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        if self
            .exclude_set
            .as_ref()
            .is_some_and(|exc| exc.is_match(path))
        {
            return true;
        }
        self.exclude_re
            .as_ref()
            .is_some_and(|re| re.is_match(path_str.as_ref()))
    }
}

/// Pre-compiled filter set for all cops + global excludes.
pub struct CopFilterSet {
    pub(crate) global_exclude: GlobSet,
    pub(crate) global_exclude_exact: Vec<globset::GlobMatcher>,
    pub(crate) global_exclude_re: Option<RegexSet>,
    pub(crate) filters: Vec<CopFilter>,
    pub(crate) name_to_index: HashMap<String, usize>,
    pub(crate) config_dir: Option<PathBuf>,
    pub(crate) base_dir: Option<PathBuf>,
    pub(crate) scan_root: Option<PathBuf>,
    pub(crate) sub_config_dirs: Vec<PathBuf>,
    pub(crate) universal_cop_indices: Vec<usize>,
    pub(crate) pattern_cop_indices: Vec<usize>,
    pub(crate) migrated_schema_version: Option<String>,
}

impl CopFilterSet {
    pub fn set_scan_root(&mut self, root: PathBuf) {
        self.scan_root = Some(root);
    }

    pub fn build(config: &ResolvedConfig, registry: &CopRegistry) -> Self {
        config.build_cop_filters(registry)
    }

    /// Global excludes only — enough for `--list-target-files` / file discovery.
    pub fn for_discover(config: &ResolvedConfig) -> Self {
        config.build_discover_filters()
    }

    pub fn is_cop_enabled_for_file(&self, name: &str, path: &Path) -> bool {
        self.name_to_index
            .get(name)
            .is_some_and(|&i| self.is_cop_match(i, path))
    }

    pub fn filter_for(&self, name: &str) -> Option<&CopFilter> {
        self.name_to_index.get(name).map(|&i| &self.filters[i])
    }

    pub fn cop_filter(&self, index: usize) -> &CopFilter {
        &self.filters[index]
    }

    pub fn universal_cop_indices(&self) -> &[usize] {
        &self.universal_cop_indices
    }

    pub fn pattern_cop_indices(&self) -> &[usize] {
        &self.pattern_cop_indices
    }

    pub(crate) fn nearest_config_dir(&self, path: &Path) -> Option<&Path> {
        for dir in &self.sub_config_dirs {
            if path.starts_with(dir) {
                return Some(dir.as_path());
            }
        }
        self.config_dir.as_deref()
    }

    pub(crate) fn matches_global_exclude_glob(&self, path: &Path) -> bool {
        self.global_exclude.is_match(path)
            && self
                .global_exclude_exact
                .iter()
                .any(|matcher| exact_glob_matches(matcher, path))
    }

    fn exclude_re_matches(&self, path: &Path) -> bool {
        self.global_exclude_re
            .as_ref()
            .is_some_and(|re| re.is_match(path.to_string_lossy().as_ref()))
    }

    fn path_globally_excluded_forms(&self, path: &Path) -> bool {
        self.matches_global_exclude_glob(path) || self.exclude_re_matches(path)
    }

    pub fn is_globally_excluded(&self, path: &Path) -> bool {
        if self.path_globally_excluded_forms(path) {
            return true;
        }
        if let Ok(stripped) = path.strip_prefix("./") {
            if self.path_globally_excluded_forms(stripped) {
                return true;
            }
        }
        self.globally_excluded_under_dirs(path)
    }

    fn globally_excluded_under_dirs(&self, path: &Path) -> bool {
        for dir in [&self.base_dir, &self.config_dir].into_iter().flatten() {
            if let Ok(rel) = path.strip_prefix(dir) {
                if self.path_globally_excluded_forms(rel) {
                    return true;
                }
            }
        }
        false
    }

    pub fn is_migrated_file(&self, path: &Path) -> bool {
        let Some(ref version) = self.migrated_schema_version else {
            return false;
        };
        first_timestamp_leq(
            path.file_name().and_then(|n| n.to_str()).unwrap_or(""),
            version,
        )
    }
}

fn first_timestamp_leq(basename: &str, version: &str) -> bool {
    let bytes = basename.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i - start >= 14 {
            return &basename[start..start + 14] <= version;
        }
    }
    false
}

#[cfg(test)]
mod prepared_exclusion_tests {
    use super::*;
    use crate::config::globutil::{extract_ruby_regexp, glob_matches};

    fn legacy_forms(filters: &CopFilterSet, patterns: &[String], path: &Path) -> bool {
        filters.global_exclude.is_match(path)
            && patterns
                .iter()
                .filter(|pattern| extract_ruby_regexp(pattern).is_none())
                .any(|pattern| glob_matches(pattern, path))
            || filters.exclude_re_matches(path)
    }
    fn legacy_excluded(filters: &CopFilterSet, patterns: &[String], path: &Path) -> bool {
        legacy_forms(filters, patterns, path)
            || path
                .strip_prefix("./")
                .is_ok_and(|path| legacy_forms(filters, patterns, path))
            || [&filters.base_dir, &filters.config_dir]
                .into_iter()
                .flatten()
                .any(|dir| {
                    path.strip_prefix(dir)
                        .is_ok_and(|path| legacy_forms(filters, patterns, path))
                })
    }

    #[test]
    fn prepared_verifiers_preserve_exact_glob_and_regex_path_forms() {
        let patterns = [
            "vendor/**/*",
            "**/tmp/**/file?.rb",
            "**/.git/**/*",
            "a/*/c.rb",
            "foo{,bar}/**/*.rb",
            "[ab]/**/*.rb",
            "[",
            "/excluded\\.rb$/i",
            "/(?i)flagged\\.rb$/",
            "/[invalid/",
            "root/**/nested/**/probe.rb",
        ];
        let paths = [
            "vendor/a.rb",
            "vendor/a/b.rb",
            "./vendor/a.rb",
            "tmp/file1.rb",
            "x/tmp/y/file2.rb",
            ".git/HEAD",
            "x/.git/objects/aa",
            "a/b/c.rb",
            "a/b/d/c.rb",
            "foo/test.rb",
            "foobar/x/test.rb",
            "a/deep/probe.rb",
            "b/test.rb",
            "excluded.rb",
            "EXCLUDED.rb",
            "FLAGGED.rb",
            "ok.rb",
            "root/nested/probe.rb",
            "root/x/nested/y/probe.rb",
            "[",
            "",
        ];
        for selection in patterns
            .iter()
            .map(|pattern| vec![pattern.to_string()])
            .chain(std::iter::once(
                patterns.iter().map(|pattern| pattern.to_string()).collect(),
            ))
        {
            let mut config = ResolvedConfig::empty();
            config.global_excludes = selection.clone();
            config.base_dir = Some(PathBuf::from("/repo"));
            config.config_dir = Some(PathBuf::from("/repo/nested"));
            let filters = CopFilterSet::for_discover(&config);
            for path in paths {
                for form in [
                    PathBuf::from(path),
                    PathBuf::from("/repo").join(path),
                    PathBuf::from("/repo/nested").join(path),
                ] {
                    assert_eq!(
                        filters.is_globally_excluded(&form),
                        legacy_excluded(&filters, &selection, &form),
                        "{selection:?} / {form:?}"
                    );
                }
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn prepared_verifiers_preserve_lossy_matching_for_non_utf8_paths() {
        use std::os::unix::ffi::OsStringExt;
        let mut config = ResolvedConfig::empty();
        config.global_excludes = vec!["ignored/*".into(), "**/�.rb".into()];
        let filters = CopFilterSet::for_discover(&config);
        for raw in [
            b"ignored/\xff.rb".to_vec(),
            b"x/\xff.rb".to_vec(),
            b"other/ok.rb".to_vec(),
        ] {
            let path = PathBuf::from(std::ffi::OsString::from_vec(raw));
            assert_eq!(
                filters.is_globally_excluded(&path),
                legacy_excluded(&filters, &config.global_excludes, &path)
            );
        }
    }

    #[test]
    fn prepared_filters_keep_inherited_and_nested_config_exclusion_policies_separate() {
        let dir = tempfile::tempdir().unwrap();
        let child = dir.path().join("nested");
        std::fs::create_dir(&child).unwrap();
        let root_yaml = dir.path().join(".rubocop.yml");
        let child_yaml = child.join(".rubocop.yml");
        std::fs::write(&root_yaml, "AllCops:\n  Exclude: ['tmp/**/*']\n").unwrap();
        std::fs::write(
            &child_yaml,
            "inherit_from: ../.rubocop.yml\nAllCops:\n  Exclude: ['generated/**/*']\n",
        )
        .unwrap();
        let root_config =
            crate::config::load_config(Some(&root_yaml), Some(dir.path()), None).unwrap();
        let child_config =
            crate::config::load_config(Some(&child_yaml), Some(&child), None).unwrap();
        let registry = CopRegistry::default_registry();
        for config in [&root_config, &child_config] {
            for filters in [
                CopFilterSet::for_discover(config),
                CopFilterSet::build(config, &registry),
            ] {
                for path in [
                    "tmp/a.rb",
                    "nested/tmp/a.rb",
                    "nested/generated/a.rb",
                    "nested/generated/deep/b.rb",
                    "lib/a.rb",
                    "./tmp/a.rb",
                ] {
                    for form in [PathBuf::from(path), dir.path().join(path)] {
                        assert_eq!(
                            filters.is_globally_excluded(&form),
                            legacy_excluded(&filters, &config.global_excludes, &form),
                            "{form:?}"
                        );
                    }
                }
            }
        }
        assert!(
            CopFilterSet::for_discover(&root_config)
                .is_globally_excluded(&dir.path().join("tmp/a.rb"))
        );
        assert!(
            CopFilterSet::for_discover(&child_config)
                .is_globally_excluded(&child.join("generated/a.rb"))
        );
        assert!(
            !CopFilterSet::for_discover(&child_config)
                .is_globally_excluded(&child.join("tmp/a.rb"))
        );
    }
}
