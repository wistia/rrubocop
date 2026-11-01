use serde::{Deserialize, Deserializer, Serializer};
use std::sync::Arc;

pub(super) fn is_empty(line: &Arc<str>) -> bool {
    line.is_empty()
}

pub(super) fn serialize<S: Serializer>(line: &Arc<str>, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(line)
}

pub(super) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Arc<str>, D::Error> {
    String::deserialize(deserializer).map(Arc::from)
}
