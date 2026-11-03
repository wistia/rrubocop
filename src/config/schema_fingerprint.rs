use super::ResolvedConfig;
use super::resolved_state::{resolve_enabled_state, state_to_enabled, version_gate_disables};
use sha2::Digest;
use std::path::Path;

impl ResolvedConfig {
    fn schema_dependency_enabled(&self, name: &str) -> bool {
        let config = self.cop_configs.get(name);
        let enabled = state_to_enabled(
            resolve_enabled_state(&self.enable_inputs(name, "Rails", config, true)),
            self.new_cops,
            self.disabled_by_default,
            true,
        ) && !version_gate_disables(self, name, "Rails", config, true);
        enabled
            || self.dir_overrides.iter().any(|(_, layer)| {
                layer.cop_configs.contains_key(name)
                    || layer.department_configs.contains_key("Rails")
            })
    }
    pub(super) fn hash_schema_dependencies(&self, hash: &mut sha2::Sha256) {
        let Ok(directory) = std::env::current_dir() else {
            return;
        };
        let adapter = if self.schema_dependency_enabled("Rails/BulkChangeTable")
            || self.schema_dependency_enabled("Rails/NotNullColumn")
        {
            crate::cop::rails::database_dependency_adapter(&directory)
        } else {
            None
        };
        self.hash_schema_dependencies_at(hash, &directory, adapter.as_deref());
    }
    fn hash_schema_dependencies_at(
        &self,
        hash: &mut sha2::Sha256,
        directory: &Path,
        adapter: Option<&str>,
    ) {
        if self.schema_dependency_enabled("Rails/UniqueValidationWithoutIndex") {
            hash.update(b"rrubocop-schema-source-v1\0");
            let bytes = crate::cop::rails::schema_dependency_path(directory)
                .and_then(|path| std::fs::read(path).ok());
            if let Some(bytes) = bytes {
                hash.update([1]);
                hash.update((bytes.len() as u64).to_le_bytes());
                hash.update(bytes);
            } else {
                hash.update([0]);
            }
        }
        if self.schema_dependency_enabled("Rails/BulkChangeTable")
            || self.schema_dependency_enabled("Rails/NotNullColumn")
        {
            hash.update(b"rrubocop-database-adapter-v1\0");
            match adapter {
                Some("mysql") => hash.update([1]),
                Some("postgresql") => hash.update([2]),
                _ => hash.update([0]),
            };
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::{CopConfig, EnabledState};
    fn configured(enabled: bool) -> ResolvedConfig {
        let mut config = ResolvedConfig::empty();
        config.disabled_by_default = true;
        for name in [
            "Rails/UniqueValidationWithoutIndex",
            "Rails/BulkChangeTable",
            "Rails/NotNullColumn",
        ] {
            config.cop_configs.insert(
                name.into(),
                CopConfig {
                    enabled: if enabled {
                        EnabledState::True
                    } else {
                        EnabledState::False
                    },
                    ..CopConfig::default()
                },
            );
        }
        config
    }
    fn fingerprint(config: &ResolvedConfig, directory: &Path, adapter: Option<&str>) -> Vec<u8> {
        let mut hash = sha2::Sha256::new();
        config.hash_schema_dependencies_at(&mut hash, directory, adapter);
        hash.finalize().to_vec()
    }
    #[test]
    fn schema_dependency_tracks_creation_same_size_edits_and_deletion() {
        let root = tempfile::tempdir().unwrap();
        let nested = root.path().join("models/nested");
        std::fs::create_dir_all(&nested).unwrap();
        let schema = root.path().join("db/schema.rb");
        std::fs::create_dir_all(schema.parent().unwrap()).unwrap();
        let config = configured(true);
        let absent = fingerprint(&config, &nested, None);
        std::fs::write(&schema, b"schema_A").unwrap();
        let first = fingerprint(&config, &nested, None);
        assert_ne!(first, absent);
        assert_eq!(first, fingerprint(&config, &nested, None));
        std::fs::write(&schema, b"schema_B").unwrap();
        assert_ne!(first, fingerprint(&config, &nested, None));
        std::fs::remove_file(&schema).unwrap();
        assert_eq!(absent, fingerprint(&config, &nested, None));
    }
    #[test]
    fn database_dependency_uses_only_normalized_adapter_and_respects_disabling() {
        let root = tempfile::tempdir().unwrap();
        let config = configured(true);
        assert_ne!(
            fingerprint(&config, root.path(), Some("mysql")),
            fingerprint(&config, root.path(), Some("postgresql"))
        );
        assert_eq!(
            fingerprint(&config, root.path(), None),
            fingerprint(
                &config,
                root.path(),
                Some("mysql2://private:secret@host/db")
            )
        );
        let disabled = configured(false);
        assert_eq!(
            fingerprint(&disabled, root.path(), Some("mysql")),
            fingerprint(&disabled, root.path(), Some("postgresql"))
        );
    }
}
