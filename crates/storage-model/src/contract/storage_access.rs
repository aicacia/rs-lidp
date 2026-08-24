use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StorageAccess {
    Read,
    Write,
    List,
    Metadata,
    Delete,
    Rename,
}
