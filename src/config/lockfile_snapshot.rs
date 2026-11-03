//! Lockfile bytes shared only while one synchronous configuration load is active.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

type Files = HashMap<PathBuf, Option<Arc<str>>>;
thread_local! {
    static ACTIVE: RefCell<Option<Files>> = const { RefCell::new(None) };
}

struct Scope {
    previous: Option<Files>,
}
impl Drop for Scope {
    fn drop(&mut self) {
        ACTIVE.with(|active| *active.borrow_mut() = self.previous.take());
    }
}

pub(super) fn with_snapshot<T>(load: impl FnOnce() -> T) -> T {
    let _scope = Scope {
        previous: ACTIVE.with(|active| active.replace(Some(Files::new()))),
    };
    load()
}

pub(super) fn read(path: &Path) -> Option<Arc<str>> {
    let read = || std::fs::read_to_string(path).ok().map(Arc::from);
    ACTIVE.with(|active| match active.borrow_mut().as_mut() {
        Some(files) => files.entry(path.to_path_buf()).or_insert_with(read).clone(),
        None => read(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_and_absence_are_stable_only_within_the_load() {
        let dir = tempfile::tempdir().unwrap();
        let existing = dir.path().join("Gemfile.lock");
        let missing = dir.path().join("gems.locked");
        std::fs::write(&existing, "before").unwrap();
        with_snapshot(|| {
            assert_eq!(read(&existing).as_deref(), Some("before"));
            assert!(read(&missing).is_none());
            std::fs::write(&existing, "after").unwrap();
            std::fs::write(&missing, "created").unwrap();
            assert_eq!(read(&existing).as_deref(), Some("before"));
            assert!(read(&missing).is_none());
            with_snapshot(|| assert_eq!(read(&existing).as_deref(), Some("after")));
            assert_eq!(read(&existing).as_deref(), Some("before"));
        });
        assert_eq!(read(&existing).as_deref(), Some("after"));
        assert_eq!(read(&missing).as_deref(), Some("created"));
        with_snapshot(|| assert_eq!(read(&existing).as_deref(), Some("after")));
    }

    #[test]
    fn independent_threads_and_unwinding_do_not_reuse_a_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Gemfile.lock");
        std::fs::write(&path, "one").unwrap();
        with_snapshot(|| {
            assert_eq!(read(&path).as_deref(), Some("one"));
            std::fs::write(&path, "two").unwrap();
            let other = path.clone();
            std::thread::spawn(move || {
                with_snapshot(|| assert_eq!(read(&other).as_deref(), Some("two")))
            })
            .join()
            .unwrap();
            assert_eq!(read(&path).as_deref(), Some("one"));
        });
        let _ = std::panic::catch_unwind(|| {
            with_snapshot(|| {
                assert_eq!(read(&path).as_deref(), Some("two"));
                panic!("load interrupted");
            })
        });
        std::fs::write(&path, "three").unwrap();
        assert_eq!(read(&path).as_deref(), Some("three"));
    }
}

#[cfg(test)]
mod consumer_tests {
    use super::*;
    use crate::config::gem_path_version::select_version;
    use crate::config::load_lockfile::{lockfile_gem_version, resolve_lockfile_meta};
    use crate::config::types::ConfigLayer;

    #[test]
    fn plugin_and_runtime_versions_share_bytes_but_preserve_per_gem_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let primary = dir.path().join("Gemfile.lock");
        let fallback = dir.path().join("gems.locked");
        std::fs::write(
            &primary,
            "GEM\n  specs:\n    rubocop (1.65.1)\n    railties (bad)\n    rack (3.1.2)\n",
        )
        .unwrap();
        std::fs::write(&fallback, "GEM\n  specs:\n    rubocop (1.77.0)\n    rubocop-rails (2.26.0)\n    railties (8.1.3.1)\n    rack (2.2.0)\n").unwrap();
        with_snapshot(|| {
            assert_eq!(select_version("rubocop", dir.path()).unwrap(), "1.65.1");
            std::fs::write(&primary, "    rubocop (1.77.0)\n    rack (2.0.0)\n").unwrap();
            assert_eq!(lockfile_gem_version(dir.path(), "rack"), Some(3.1));
            assert_eq!(
                select_version("rubocop-rails", dir.path()).unwrap(),
                "2.26.0"
            );
            let base = ConfigLayer {
                target_rails_version: Some(7.2),
                ..ConfigLayer::empty()
            };
            let meta = resolve_lockfile_meta(&base, dir.path());
            assert_eq!(meta.target_rails_version, Some(7.2));
            assert_eq!(meta.railties_version, Some(8.1));
            assert!(meta.railties_in_lockfile);
            assert_eq!(meta.rack_version, Some(3.1));
        });
        assert_eq!(select_version("rubocop", dir.path()).unwrap(), "1.77.0");
        assert_eq!(lockfile_gem_version(dir.path(), "rack"), Some(2.0));
    }

    #[test]
    fn failed_reads_are_shared_without_losing_fallback_or_directory_identity() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        std::fs::write(first.path().join("Gemfile.lock"), [0xff]).unwrap();
        std::fs::write(first.path().join("gems.locked"), "    rack (3.1.2)\n").unwrap();
        std::fs::write(second.path().join("Gemfile.lock"), "    rack (2.2.0)\n").unwrap();
        with_snapshot(|| {
            assert_eq!(lockfile_gem_version(first.path(), "rack"), Some(3.1));
            std::fs::write(first.path().join("Gemfile.lock"), "    rack (4.0.0)\n").unwrap();
            assert_eq!(lockfile_gem_version(first.path(), "rack"), Some(3.1));
            assert_eq!(lockfile_gem_version(second.path(), "rack"), Some(2.2));
        });
        assert_eq!(lockfile_gem_version(first.path(), "rack"), Some(4.0));
    }

    #[test]
    fn independent_public_loads_refresh_versions_including_after_config_errors() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join(".rubocop.yml");
        let lock = dir.path().join("Gemfile.lock");
        std::fs::write(&config, "require: rubocop-rails\n").unwrap();
        std::fs::write(&lock, "    rubocop (1.65.1)\n    rubocop-rails (2.26.0)\n    railties (7.2.0)\n    rack (3.1.2)\n").unwrap();
        let first = crate::config::load_config(Some(&config), Some(dir.path()), None).unwrap();
        assert_eq!(first.rubocop_version.as_deref(), Some("1.65.1"));
        assert_eq!(first.railties_version, Some(7.2));
        std::fs::write(&config, "AllCops: [\n").unwrap();
        assert!(crate::config::load_config(Some(&config), Some(dir.path()), None).is_err());
        std::fs::write(&config, "require: rubocop-rails\n").unwrap();
        std::fs::write(&lock, "    rubocop (1.77.0)\n    rubocop-rails (2.32.0)\n    railties (8.1.3.1)\n    rack (2.2.0)\n").unwrap();
        let next = crate::config::load_config(Some(&config), Some(dir.path()), None).unwrap();
        assert_eq!(next.rubocop_version.as_deref(), Some("1.77.0"));
        assert_eq!(next.rubocop_rails_version.as_deref(), Some("2.32.0"));
        assert_eq!(next.railties_version, Some(8.1));
        assert_eq!(next.rack_version, Some(2.2));
        let defaults = crate::config::load_default_config(Some(dir.path()), None);
        assert_eq!(defaults.rubocop_version.as_deref(), Some("1.77.0"));
    }
}
