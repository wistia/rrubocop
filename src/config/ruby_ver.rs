//! Ruby / gem version resolution from gemspec and lockfiles.

use super::types::ConfigLayer;
use std::path::Path;

const KNOWN_RUBIES: &[f64] = &[
    2.0, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 3.0, 3.1, 3.2, 3.3, 3.4, 4.0, 4.1,
];

fn single_gemspec(config_dir: &Path) -> Option<std::path::PathBuf> {
    let entries: Vec<_> = std::fs::read_dir(config_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "gemspec"))
        .collect();
    (entries.len() == 1).then(|| entries[0].path())
}

fn quoted_constraint(after_eq: &str) -> Option<&str> {
    let quote_start = after_eq.find(['\'', '"'])?;
    let qc = after_eq.as_bytes()[quote_start] as char;
    let rest = &after_eq[quote_start + 1..];
    let quote_end = rest.find(qc)?;
    Some(&rest[..quote_end])
}

fn parse_major(digits: &[&str]) -> Option<u64> {
    digits.first()?.parse().ok()
}

fn parse_minor(digits: &[&str]) -> Option<u64> {
    digits.get(1)?.parse().ok()
}

fn major_minor_f64(ver_str: &str) -> Option<f64> {
    let digits: Vec<&str> = ver_str.split('.').collect();
    let major = parse_major(&digits)?;
    let minor = parse_minor(&digits)?;
    Some(major as f64 + minor as f64 / 10.0)
}

fn min_version_from_constraint(constraint: &str) -> Option<f64> {
    major_minor_f64(constraint.trim_start_matches(|c: char| !c.is_ascii_digit()))
}

fn parse_required_ruby_line(trimmed: &str) -> Option<f64> {
    if trimmed.starts_with('#') || !trimmed.contains(".required_ruby_version") {
        return None;
    }
    let after = trimmed.split(".required_ruby_version").nth(1)?;
    let after = after.trim_start();
    if !after.starts_with('=') {
        return None;
    }
    let constraint = quoted_constraint(after)?;
    let min_version = min_version_from_constraint(constraint)?;
    KNOWN_RUBIES.iter().copied().find(|&v| v >= min_version)
}

/// Resolve TargetRubyVersion from a gemspec's `required_ruby_version`.
pub(crate) fn resolve_ruby_version_from_gemspec(config_dir: &Path) -> Option<f64> {
    let path = single_gemspec(config_dir)?;
    let content = std::fs::read_to_string(path).ok()?;
    content.lines().find_map(parse_required_ruby_line)
}

fn lock_version_str<'a>(trimmed: &'a str, gem_name: &str) -> Option<&'a str> {
    let rest = trimmed.strip_prefix(gem_name)?;
    rest.strip_prefix(" (")?.strip_suffix(')')
}

fn parse_lock_version_line(trimmed: &str, gem_name: &str) -> Option<f64> {
    major_minor_f64(lock_version_str(trimmed, gem_name)?)
}

/// Parse a gem's major.minor version from Gemfile.lock / gems.locked.
pub(crate) fn parse_gem_version_from_lockfile(content: &str, gem_name: &str) -> Option<f64> {
    content
        .lines()
        .find_map(|line| parse_lock_version_line(line.trim(), gem_name))
}

fn parse_ruby_version_file(content: &str) -> Option<f64> {
    let parts: Vec<&str> = content
        .trim()
        .trim_start_matches("ruby-")
        .split('.')
        .collect();
    if parts.len() < 2 {
        return None;
    }
    let major: u64 = parts[0].parse().ok()?;
    let minor: u64 = parts[1].parse().ok()?;
    Some(major as f64 + minor as f64 / 10.0)
}

fn tool_versions_ruby(config_dir: &Path) -> Option<f64> {
    config_dir.ancestors().find_map(|dir| {
        std::fs::read_to_string(dir.join(".tool-versions"))
            .ok()?
            .lines()
            .find_map(|line| line.strip_prefix("ruby ").and_then(parse_ruby_version_file))
    })
}

pub(crate) fn resolve_target_ruby_version(base: &ConfigLayer, config_dir: &Path) -> Option<f64> {
    base.target_ruby_version
        .or_else(|| resolve_ruby_version_from_gemspec(config_dir))
        .or_else(|| {
            std::fs::read_to_string(config_dir.join(".ruby-version"))
                .ok()
                .and_then(|c| parse_ruby_version_file(&c))
        })
        .or_else(|| tool_versions_ruby(config_dir))
}

#[cfg(test)]
mod tests {
    #[test]
    fn tool_versions_sets_target_ruby() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".tool-versions"),
            "nodejs 24.0.0\nruby 3.4.9\n",
        )
        .unwrap();
        let config = super::super::types::ConfigLayer::empty();
        assert_eq!(
            super::super::load_resolve::resolve_target_ruby_version(&config, dir.path()),
            Some(3.4)
        );
    }
}
