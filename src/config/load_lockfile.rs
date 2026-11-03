//! Gemfile.lock / gems.locked version metadata for config resolution.

use std::path::Path;

use super::ruby_ver::parse_gem_version_from_lockfile;
use super::types::ConfigLayer;

pub(crate) fn lockfile_gem_version(lockfile_dir: &Path, gem_name: &str) -> Option<f64> {
    for lock_name in &["Gemfile.lock", "gems.locked"] {
        if let Some(content) = super::lockfile_snapshot::read(&lockfile_dir.join(lock_name)) {
            if let Some(ver) = parse_gem_version_from_lockfile(&content, gem_name) {
                return Some(ver);
            }
        }
    }
    None
}

pub(crate) struct LockfileMeta {
    pub target_rails_version: Option<f64>,
    pub railties_in_lockfile: bool,
    pub railties_version: Option<f64>,
    pub rack_version: Option<f64>,
}

fn railties_from_lock(lockfile_dir: &Path) -> Option<f64> {
    lockfile_gem_version(lockfile_dir, "railties")
}

pub(crate) fn resolve_lockfile_meta(base: &ConfigLayer, lockfile_dir: &Path) -> LockfileMeta {
    let mut railties_version = None;
    let mut railties_in_lockfile = false;
    let target_rails_version = base.target_rails_version.or_else(|| {
        let ver = railties_from_lock(lockfile_dir)?;
        railties_in_lockfile = true;
        railties_version = Some(ver);
        Some(ver)
    });
    if !railties_in_lockfile && base.target_rails_version.is_some() {
        if let Some(ver) = railties_from_lock(lockfile_dir) {
            railties_in_lockfile = true;
            railties_version = Some(ver);
        }
    }
    LockfileMeta {
        target_rails_version,
        railties_in_lockfile,
        railties_version,
        rack_version: lockfile_gem_version(lockfile_dir, "rack"),
    }
}
