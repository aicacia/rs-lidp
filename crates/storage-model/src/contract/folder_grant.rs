use alloc::{string::String, vec::Vec};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::StorageAccess;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderGrant {
    pub root: String,
    pub access: Vec<StorageAccess>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_bytes: Option<u64>,
}
