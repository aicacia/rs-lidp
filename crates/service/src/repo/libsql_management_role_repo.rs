use std::sync::Arc;

use libsql::{Database, de::from_row};
use model::model::{ManagementRole, ManagementUserRole};

use crate::repo::{ManagementRoleRepo, RepoResult};

pub struct LibSqlManagementRoleRepo {
    database: Arc<Database>,
}

impl LibSqlManagementRoleRepo {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl ManagementRoleRepo for LibSqlManagementRoleRepo {
    async fn list_roles(&self, offset: u32, limit: u32) -> RepoResult<Vec<ManagementRole>> {
        let connection = self.database.connect()?;
        let query = r#"
            SELECT
                id,
                name,
                description,
                created_at,
                updated_at
            FROM management_roles
            ORDER BY name ASC
            LIMIT ? OFFSET ?
        "#;

        let mut rows = connection
            .query(query, libsql::params![i64::from(limit), i64::from(offset)])
            .await?;
        let mut roles = Vec::new();

        while let Some(row) = rows.next().await? {
            roles.push(from_row::<ManagementRole>(&row)?);
        }

        Ok(roles)
    }

    async fn create_role(&self, name: &str, description: Option<&str>) -> RepoResult<ManagementRole> {
        let connection = self.database.connect()?;
        let query = r#"
            INSERT INTO management_roles (name, description)
            VALUES (?, ?)
            RETURNING
                id,
                name,
                description,
                created_at,
                updated_at
        "#;

        let mut rows = connection
            .query(query, libsql::params![name, description])
            .await?;

        let row = rows
            .next()
            .await?
            .ok_or_else(|| libsql::Error::Misuse("role not found after insert".into()))?;

        Ok(from_row::<ManagementRole>(&row)?)
    }

    async fn find_role_by_id(&self, role_id: i64) -> RepoResult<Option<ManagementRole>> {
        let connection = self.database.connect()?;
        let query = r#"
            SELECT
                id,
                name,
                description,
                created_at,
                updated_at
            FROM management_roles
            WHERE id = ?
        "#;

        let row = {
            let mut rows = connection.query(query, libsql::params![role_id]).await?;
            if let Some(row) = rows.next().await? {
                row
            } else {
                return Ok(None);
            }
        };

        Ok(Some(from_row::<ManagementRole>(&row)?))
    }

    async fn delete_role_by_id(&self, role_id: i64) -> RepoResult<()> {
        let connection = self.database.connect()?;
        connection
            .execute(
                "DELETE FROM management_roles WHERE id = ?",
                libsql::params![role_id],
            )
            .await?;
        Ok(())
    }

    async fn list_user_roles(&self, user_id: i64) -> RepoResult<Vec<ManagementUserRole>> {
        let connection = self.database.connect()?;
        let query = r#"
            SELECT
                ur.id,
                ur.user_id,
                ur.role_id,
                r.name AS role_name,
                ur.created_at,
                ur.updated_at
            FROM management_user_roles ur
            JOIN management_roles r ON r.id = ur.role_id
            WHERE ur.user_id = ?
            ORDER BY r.name ASC
        "#;

        let mut rows = connection.query(query, libsql::params![user_id]).await?;
        let mut assignments = Vec::new();

        while let Some(row) = rows.next().await? {
            assignments.push(from_row::<ManagementUserRole>(&row)?);
        }

        Ok(assignments)
    }

    async fn assign_role_to_user(&self, user_id: i64, role_id: i64) -> RepoResult<()> {
        let connection = self.database.connect()?;
        connection
            .execute(
                r#"
                    INSERT INTO management_user_roles (user_id, role_id)
                    VALUES (?, ?)
                    ON CONFLICT(user_id, role_id)
                    DO UPDATE SET updated_at = unixepoch()
                "#,
                libsql::params![user_id, role_id],
            )
            .await?;
        Ok(())
    }

    async fn revoke_role_from_user(&self, user_id: i64, role_id: i64) -> RepoResult<()> {
        let connection = self.database.connect()?;
        connection
            .execute(
                "DELETE FROM management_user_roles WHERE user_id = ? AND role_id = ?",
                libsql::params![user_id, role_id],
            )
            .await?;
        Ok(())
    }

    async fn count_user_role_assignments(&self) -> RepoResult<u64> {
        let connection = self.database.connect()?;
        let mut rows = connection
            .query("SELECT COUNT(*) AS count FROM management_user_roles", ())
            .await?;

        if let Some(row) = rows.next().await? {
            let count = row.get::<i64>(0)?;
            Ok(count.max(0) as u64)
        } else {
            Ok(0)
        }
    }
}
