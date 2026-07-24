use std::sync::Arc;

use chrono::{DateTime, Utc};
use libsql::{Database, de::from_row};
use model::{contract::EntityType, model::Key};

use crate::repo::{KeyRepo, RepoResult};

pub struct LibSqlKeyRepo {
    database: Arc<Database>,
}

impl LibSqlKeyRepo {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl KeyRepo for LibSqlKeyRepo {
    async fn list_active(&self) -> RepoResult<Vec<Key>> {
        let conn = self.database.connect()?;
        let query = r#"
            SELECT
                id,
                entity_type,
                entity_id,
                version,
                derivation_path,
                name,
                hardened,
                revoked_at,
                expires_at,
                created_at,
                updated_at
            FROM keys
            WHERE (revoked_at IS NULL OR revoked_at > unixepoch())
                AND (expires_at IS NULL OR expires_at > unixepoch())
        "#;

        let mut rows = conn.query(query, libsql::params![]).await?;
        let mut keys = Vec::new();

        while let Some(row) = rows.next().await? {
            keys.push(from_row::<Key>(&row)?);
        }

        Ok(keys)
    }

    async fn find_by_id(&self, id: i64) -> RepoResult<Option<Key>> {
        let conn = self.database.connect()?;
        let query = r#"
            SELECT
                id,
                entity_type,
                entity_id,
                version,
                derivation_path,
                name,
                hardened,
                revoked_at,
                expires_at,
                created_at,
                updated_at
            FROM keys
            WHERE id = ?
                AND (revoked_at IS NULL OR revoked_at > unixepoch())
                AND (expires_at IS NULL OR expires_at > unixepoch())
            LIMIT 1
        "#;

        let mut rows = conn.query(query, libsql::params![id]).await?;
        if let Some(row) = rows.next().await? {
            Ok(Some(from_row::<Key>(&row)?))
        } else {
            Ok(None)
        }
    }

    async fn active_by_entity_type_and_id(
        &self,
        entity_type: EntityType,
        entity_id: i64,
    ) -> RepoResult<Option<Key>> {
        let conn = self.database.connect()?;
        let query = r#"
            SELECT
                id,
                entity_type,
                entity_id,
                version,
                derivation_path,
                name,
                hardened,
                revoked_at,
                expires_at,
                created_at,
                updated_at
            FROM keys
            WHERE entity_type = ? AND entity_id = ?
                AND (revoked_at IS NULL OR revoked_at > unixepoch())
                AND (expires_at IS NULL OR expires_at > unixepoch())
            LIMIT 1
        "#;

        let mut rows = conn
            .query(query, libsql::params![entity_type as i64, entity_id])
            .await?;
        if let Some(row) = rows.next().await? {
            Ok(Some(from_row::<Key>(&row)?))
        } else {
            Ok(None)
        }
    }

    async fn create_key(
        &self,
        entity_type: EntityType,
        entity_id: i64,
        hardened: bool,
        name: String,
        expires_at: Option<DateTime<Utc>>,
    ) -> RepoResult<Key> {
        let conn = self.database.connect()?;
        let mut tx = conn.transaction().await?;
        let key = self
            .tx_create_key(entity_type, entity_id, hardened, name, expires_at, &mut tx)
            .await?;
        tx.commit().await?;
        Ok(key)
    }
}

impl LibSqlKeyRepo {
    pub async fn tx_next_version(
        &self,
        entity_type: EntityType,
        entity_id: i64,
        tx: &mut libsql::Transaction,
    ) -> RepoResult<i64> {
        let query = r#"
            SELECT version + 1 AS next_version
            FROM keys
            WHERE entity_id = ? AND entity_type = ?
            ORDER BY version DESC
            LIMIT 1;
        "#;

        let mut rows = tx
            .query(query, libsql::params![entity_id, entity_type as i64])
            .await?;

        if let Some(row) = rows.next().await? {
            let next_version: i64 = row.get(0)?;
            Ok(next_version)
        } else {
            Ok(0)
        }
    }

    pub async fn tx_create_key(
        &self,
        entity_type: EntityType,
        entity_id: i64,
        hardened: bool,
        name: String,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
        tx: &mut libsql::Transaction,
    ) -> RepoResult<Key> {
        let version = self.tx_next_version(entity_type, entity_id, tx).await?;

        let derivation_path = Key::build_derivation_path(entity_type, entity_id, version, hardened);

        let query = r#"
            INSERT INTO keys (entity_type, entity_id, version, derivation_path, name, hardened, expires_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            RETURNING *;
        "#;

        let mut rows = tx
            .query(
                query,
                libsql::params![
                    entity_type as u32,
                    entity_id,
                    version,
                    derivation_path,
                    name,
                    hardened,
                    expires_at.map(|dt| dt.timestamp())
                ],
            )
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(from_row::<Key>(&row)?)
        } else {
            Err(libsql::Error::NullValue.into())
        }
    }
}
