//! Storage core of the result cache: the single redb table, the JSON
//! entry codec, and the recency-retention policy that bounds the table
//! to [`MAX_ENTRIES`] newest rows.
//!
//! Same layout style as original abcop.

use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(test)]
use redb::ReadableTableMetadata;
use redb::{Database, Durability, ReadableDatabase, ReadableTable, TableDefinition};

use crate::diagnostic::Diagnostic;

/// Bump whenever cops or stored diagnostic shape change so stale entries
/// are never served.
pub(crate) const RULES_REV: u32 = 20;

pub(crate) const MAX_ENTRIES: usize = 20_000;
const ENTRIES: TableDefinition<&str, &[u8]> = TableDefinition::new("entries");

pub(crate) struct EntryStore {
    pub(super) db: Database,
}

impl EntryStore {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    pub(crate) fn get(&self, key: &str) -> Option<CachedDiags> {
        let rtx = self.db.begin_read().ok()?;
        let table = rtx.open_table(ENTRIES).ok()?;
        let value = table.get(key).ok()??;
        let f: CachedFile = serde_json::from_slice(value.value()).ok()?;
        Some(f.diagnostics)
    }

    pub(crate) fn store(&self, key: &str, diagnostics: &[Diagnostic]) {
        let payload = CachedFileRef {
            ts: now_ms(),
            diagnostics,
        };
        let Ok(bytes) = serde_json::to_vec(&payload) else {
            return;
        };
        let Some(tx) = self.write_tx() else {
            return;
        };
        let Ok(mut table) = tx.open_table(ENTRIES) else {
            return;
        };
        if table.insert(key, bytes.as_slice()).is_err() {
            return;
        }
        drop(table);
        let _ = tx.commit();
    }

    /// Keep the newest MAX_ENTRIES entries; drop the rest.
    pub(crate) fn prune(&self) {
        let Some(by_age) = self.entries_by_age() else {
            return;
        };
        if by_age.len() <= MAX_ENTRIES {
            return;
        }
        let mut newest_first = by_age;
        newest_first.sort_by_key(|(t, _)| std::cmp::Reverse(*t));
        self.remove_keys(
            &newest_first
                .iter()
                .skip(MAX_ENTRIES)
                .map(|(_, k)| k.clone())
                .collect::<Vec<_>>(),
        );
    }

    fn entries_by_age(&self) -> Option<Vec<(u64, String)>> {
        // Chained in the tail expression so the read-txn/table temporaries
        // outlive the iterator borrow.
        Some(
            self.db
                .begin_read()
                .ok()?
                .open_table(ENTRIES)
                .ok()?
                .iter()
                .ok()?
                .flatten()
                .filter_map(|(k, v)| parse_age(k.value(), v.value()))
                .collect(),
        )
    }

    fn remove_keys(&self, keys: &[String]) {
        let Some(tx) = self.write_tx() else {
            return;
        };
        let Ok(mut table) = tx.open_table(ENTRIES) else {
            return;
        };
        for key in keys {
            let _ = table.remove(key.as_str());
        }
        drop(table);
        let _ = tx.commit();
    }

    /// Best-effort write txn; `None` on lock/IO failure (skip cache write).
    pub(super) fn write_tx(&self) -> Option<redb::WriteTransaction> {
        let mut tx = self.db.begin_write().ok()?;
        tx.set_durability(Durability::None).ok()?;
        Some(tx)
    }

    #[cfg(test)]
    pub(super) fn raw_get(&self, key: &str) -> Option<usize> {
        // Chained in the tail expression so the read-txn/table temporaries
        // outlive the guard borrow.
        self.db
            .begin_read()
            .unwrap()
            .open_table(ENTRIES)
            .unwrap()
            .get(key)
            .unwrap()
            .map(|_| 1_usize)
    }

    #[cfg(test)]
    pub(super) fn raw_len(&self) -> usize {
        self.db
            .begin_read()
            .unwrap()
            .open_table(ENTRIES)
            .unwrap()
            .len()
            .unwrap() as usize
    }

    #[cfg(test)]
    pub(super) fn raw_insert(&self, key: &str, payload: &[u8]) {
        let tx = self.write_tx().expect("test write txn");
        let mut table = tx.open_table(ENTRIES).unwrap();
        table.insert(key, payload).unwrap();
        drop(table);
        tx.commit().unwrap();
    }
}

#[derive(serde::Deserialize)]
struct CachedFile {
    #[allow(dead_code)]
    ts: u64,
    diagnostics: Vec<Diagnostic>,
}

#[derive(serde::Serialize)]
struct CachedFileRef<'a> {
    ts: u64,
    diagnostics: &'a [Diagnostic],
}

pub type CachedDiags = Vec<Diagnostic>;

fn parse_age(key: &str, payload: &[u8]) -> Option<(u64, String)> {
    serde_json::from_slice::<CachedFile>(payload)
        .ok()
        .map(|f| (f.ts, key.to_string()))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
