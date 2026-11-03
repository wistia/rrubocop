use crate::cop::CopConfig;
use crate::parse::source::SourceFile;

fn adapter_database(adapter: &str) -> Option<String> {
    match adapter {
        "mysql2" | "trilogy" => Some("mysql".into()),
        "postgresql" | "postgis" => Some("postgresql".into()),
        _ => None,
    }
}
fn yaml_database(text: &str) -> Option<String> {
    let mut yaml: serde_yml::Value = serde_yml::from_str(text).ok()?;
    yaml.apply_merge().ok()?;
    let development = yaml.get("development")?.as_mapping()?;
    let adapter = development
        .get(serde_yml::Value::String("adapter".into()))
        .or_else(|| development.values().next()?.get("adapter"))?
        .as_str()?;
    adapter_database(adapter)
}
fn url_database(url: &str) -> Option<String> {
    if url.starts_with("mysql2://") || url.starts_with("trilogy://") {
        Some("mysql".into())
    } else if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        Some("postgresql".into())
    } else {
        None
    }
}
pub(super) fn database(source: &SourceFile, config: &CopConfig) -> Option<String> {
    if let Some(value) = config.options.get("Database") {
        if !value.is_null() && value.as_bool() != Some(false) {
            return Some(value.as_str().unwrap_or("").into());
        }
    }
    struct DatabaseSnapshot(Option<String>);
    let path = std::env::current_dir().ok()?.join("config/database.yml");
    source
        .operation()
        .cached(&path, || {
            DatabaseSnapshot(
                std::fs::read_to_string(&path)
                    .ok()
                    .and_then(|text| yaml_database(&text))
                    .or_else(|| {
                        std::env::var("DATABASE_URL")
                            .ok()
                            .and_then(|url| url_database(&url))
                    }),
            )
        })
        .0
        .clone()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[derive(serde::Deserialize)]
    struct Observation {
        yaml: Option<String>,
        url: Option<String>,
        database: Option<String>,
    }
    #[test]
    fn database_resolution_matches_pinned_original() {
        let rows: Vec<Observation> = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/schema-upstream/database-resolution.json"
        )))
        .unwrap();
        for row in rows {
            let database = row
                .yaml
                .as_deref()
                .and_then(yaml_database)
                .or_else(|| row.url.as_deref().and_then(url_database));
            assert_eq!(
                database, row.database,
                "yaml={:?} url={:?}",
                row.yaml, row.url
            );
        }
    }
}
