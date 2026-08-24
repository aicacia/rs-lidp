use alloc::{string::String, vec::Vec};

use serde::{Deserialize, Serialize};

use super::FolderGrant;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoragePolicy {
    pub principal: String,
    pub policy_version: u64,
    pub folders: Vec<FolderGrant>,
}
