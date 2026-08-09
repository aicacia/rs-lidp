use model::model::{Permission, Role};

use crate::repo::RepoResult;

pub trait RoleRepo {
    fn list_roles(
        &self,
        application_id: &str,
        offset: u32,
        limit: u32,
    ) -> impl Future<Output = RepoResult<Vec<Role>>>;

    fn create_role(
        &self,
        application_id: &str,
        name: &str,
        description: Option<&str>,
    ) -> impl Future<Output = RepoResult<Role>>;

    fn find_role_by_id(
        &self,
        application_id: &str,
        role_id: i64,
    ) -> impl Future<Output = RepoResult<Option<Role>>>;

    fn delete_role_by_id(
        &self,
        application_id: &str,
        role_id: i64,
    ) -> impl Future<Output = RepoResult<()>>;

    fn add_role_to_user(
        &self,
        application_id: &str,
        user_id: i64,
        role_id: i64,
    ) -> impl Future<Output = RepoResult<()>>;

    fn remove_role_from_user(
        &self,
        application_id: &str,
        user_id: i64,
        role_id: i64,
    ) -> impl Future<Output = RepoResult<()>>;

    fn list_user_roles(
        &self,
        application_id: &str,
        user_id: i64,
    ) -> impl Future<Output = RepoResult<Vec<Role>>>;

    fn list_user_permissions(
        &self,
        application_id: &str,
        user_id: i64,
    ) -> impl Future<Output = RepoResult<Vec<Permission>>>;
}
