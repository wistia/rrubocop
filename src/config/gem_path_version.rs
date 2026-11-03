//! Lockfile / baseline version selection for vendored gem configs.

use std::path::Path;

use anyhow::Result;

use super::gem_configs;
use super::gem_path_version_override::{is_well_formed_version, rubocop_version_override};
use crate::baseline;

/// Pick embedded config version for `gem_name` from lockfile / baseline.
pub(crate) fn select_version(gem_name: &str, working_dir: &Path) -> Result<String> {
    let available = gem_configs::versions_for(gem_name);
    if available.is_empty() {
        anyhow::bail!(
            "Gem '{gem_name}' has no vendored RuboCop config in this rrubocop build. \
             Add it to src/resources/gem_configs_manifest.json and re-run \
             scripts/fetch_gem_configs.py."
        );
    }
    if gem_name == "rubocop" {
        if let Some(requested) = rubocop_version_override() {
            if !is_well_formed_version(&requested) {
                anyhow::bail!("invalid --rubocop-version '{requested}': expected x.y.z");
            }
            return Ok(pick_locked_version(
                gem_name,
                &requested,
                &available,
                &format!("--rubocop-version {requested}"),
            ));
        }
    }
    if let Some(locked) = lockfile_gem_version_str(working_dir, gem_name) {
        return Ok(pick_locked_version(
            gem_name,
            &locked,
            &available,
            "Gemfile.lock",
        ));
    }
    Ok(pick_unlocked_version(gem_name, &available))
}

fn pick_locked_version(gem_name: &str, locked: &str, available: &[String], source: &str) -> String {
    if available.iter().any(|v| v == locked) {
        return locked.to_string();
    }
    if let Some(alias) = gem_configs::same_as(gem_name, locked) {
        if available.iter().any(|v| v == alias) {
            return alias.to_string();
        }
    }
    let nearest = nearest_version(locked, available).expect("available non-empty");
    eprintln!("warning: {source} has {gem_name} {locked}, using vendored config {nearest}");
    nearest
}

fn pick_unlocked_version(gem_name: &str, available: &[String]) -> String {
    baseline_version(gem_name)
        .filter(|base| available.iter().any(|v| v == base))
        .unwrap_or_else(|| available.last().cloned().unwrap())
}

fn baseline_version(gem_name: &str) -> Option<String> {
    if gem_name == "rubocop" {
        return Some(baseline::RUBOCOP.to_string());
    }
    baseline::GEMS
        .iter()
        .find(|(n, _)| *n == gem_name)
        .map(|(_, v)| (*v).to_string())
}

/// Full `x.y.z` (or longer) version from Gemfile.lock / gems.locked.
fn lockfile_gem_version_str(working_dir: &Path, gem_name: &str) -> Option<String> {
    for lock_name in &["Gemfile.lock", "gems.locked"] {
        let Some(content) = super::lockfile_snapshot::read(&working_dir.join(lock_name)) else {
            continue;
        };
        if let Some(ver) = parse_gem_version_str(&content, gem_name) {
            return Some(ver);
        }
    }
    None
}

fn parse_gem_version_str(content: &str, gem_name: &str) -> Option<String> {
    content.lines().find_map(|line| {
        let rest = line.trim().strip_prefix(gem_name)?;
        let ver = rest.strip_prefix(" (")?.strip_suffix(')')?;
        if ver.is_empty() || !ver.as_bytes()[0].is_ascii_digit() {
            return None;
        }
        Some(ver.to_string())
    })
}

fn parse_semver_part(raw: &str) -> u64 {
    raw.split(|c: char| !c.is_ascii_digit())
        .next()
        .unwrap_or("0")
        .parse()
        .unwrap_or(0)
}

fn parse_semver(v: &str) -> Option<(u64, u64, u64)> {
    let mut parts = v.split('.');
    let major = parts.next()?.parse().ok()?;
    Some((
        major,
        parse_semver_part(parts.next().unwrap_or("0")),
        parse_semver_part(parts.next().unwrap_or("0")),
    ))
}

fn version_distance(a: &str, b: &str) -> u64 {
    let Some((am, an, ap)) = parse_semver(a) else {
        return u64::MAX;
    };
    let Some((bm, bn, bp)) = parse_semver(b) else {
        return u64::MAX;
    };
    am.abs_diff(bm)
        .saturating_mul(1_000_000)
        .saturating_add(an.abs_diff(bn).saturating_mul(1_000))
        .saturating_add(ap.abs_diff(bp))
}

fn better_le(candidate: (u64, u64, u64), prev: &str) -> bool {
    parse_semver(prev).is_some_and(|p| candidate > p)
}

fn nearest_version(wanted: &str, available: &[String]) -> Option<String> {
    let mut le: Option<&String> = None;
    let mut best_any: Option<&String> = None;
    let mut best_dist = u64::MAX;
    let wanted_sem = parse_semver(wanted);
    for v in available {
        let dist = version_distance(wanted, v);
        if dist < best_dist {
            best_dist = dist;
            best_any = Some(v);
        }
        if let (Some(w), Some(c)) = (wanted_sem, parse_semver(v)) {
            if c <= w {
                let replace = match le {
                    None => true,
                    Some(prev) => better_le(c, prev),
                };
                if replace {
                    le = Some(v);
                }
            }
        }
    }
    le.or(best_any).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locked_version_selection_prefers_exact_same_as_then_nearest() {
        let available = vec![
            "1.77.0".to_string(),
            "1.79.0".to_string(),
            "1.91.0".to_string(),
        ];
        assert_eq!(
            pick_locked_version("rubocop", "1.79.0", &available, "Gemfile.lock"),
            "1.79.0"
        );
        assert_eq!(
            pick_locked_version("rubocop", "1.79.2", &available, "Gemfile.lock"),
            "1.79.0"
        );
        assert_eq!(
            pick_locked_version("rubocop", "1.93.0", &available, "Gemfile.lock"),
            "1.91.0"
        );
    }

    #[test]
    fn parse_lock_version() {
        let lock = "GEM\n  specs:\n    rubocop (1.77.0)\n    rubocop-rails (2.32.0)\n";
        assert_eq!(
            parse_gem_version_str(lock, "rubocop").as_deref(),
            Some("1.77.0")
        );
        assert_eq!(
            parse_gem_version_str(lock, "rubocop-rails").as_deref(),
            Some("2.32.0")
        );
        assert!(parse_gem_version_str(lock, "missing").is_none());
    }

    #[test]
    fn nearest_prefers_le() {
        let avail = vec!["2.32.0".into(), "2.34.3".into()];
        assert_eq!(nearest_version("2.33.0", &avail).as_deref(), Some("2.32.0"));
        assert_eq!(nearest_version("2.34.3", &avail).as_deref(), Some("2.34.3"));
        assert_eq!(nearest_version("2.30.0", &avail).as_deref(), Some("2.32.0"));
    }

    #[test]
    fn pick_locked_prefers_same_as() {
        let avail = vec!["1.77.0".into(), "1.79.0".into(), "1.84.2".into()];
        assert_eq!(
            pick_locked_version("rubocop", "1.79.2", &avail, "Gemfile.lock"),
            "1.79.0"
        );
        assert_eq!(
            pick_locked_version("rubocop", "1.80.2", &avail, "Gemfile.lock"),
            "1.79.0"
        );
        assert_eq!(
            pick_locked_version("rubocop", "1.79.0", &avail, "Gemfile.lock"),
            "1.79.0"
        );
    }
}
