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
    #[cfg(test)]
    if config.options.contains_key("SchemaDatabaseYAML")
        || config.options.contains_key("SchemaDatabaseURL")
    {
        let yaml = config
            .options
            .get("SchemaDatabaseYAML")
            .and_then(|yaml| serde_yml::to_string(yaml).ok())
            .and_then(|text| yaml_database(&text));
        let url = if let Some(value) = config.options.get("SchemaDatabaseURL") {
            value.as_str().map(str::to_owned)
        } else {
            std::env::var("DATABASE_URL").ok()
        };
        return yaml.or_else(|| url.as_deref().and_then(url_database));
    }
    struct DatabaseSnapshot(Option<String>);
    let directory = std::env::current_dir().ok()?;
    let path = directory.join("config/database.yml");
    source
        .operation()
        .cached(&path, || DatabaseSnapshot(from_directory(&directory)))
        .0
        .clone()
}
pub(crate) fn from_directory(directory: &std::path::Path) -> Option<String> {
    std::fs::read_to_string(directory.join("config/database.yml"))
        .ok()
        .and_then(|text| yaml_database(&text))
        .or_else(|| {
            std::env::var("DATABASE_URL")
                .ok()
                .and_then(|url| url_database(&url))
        })
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
