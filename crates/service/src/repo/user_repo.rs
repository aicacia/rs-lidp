use model::model::{User, UserEmail, UserPassword, UserPhoneNumber};

use crate::repo::RepoResult;

pub trait UserRepo {
    fn find_user_by_id(&self, id: i64) -> impl Future<Output = RepoResult<Option<User>>>;

    fn find_user_by_username_or_email(
        &self,
        identifier: &str,
    ) -> impl Future<Output = RepoResult<Option<User>>>;

    fn find_user_emails_by_user_id(
        &self,
        user_id: i64,
    ) -> impl Future<Output = RepoResult<Vec<UserEmail>>>;
    fn find_user_phone_numbers_by_user_id(
        &self,
        user_id: i64,
    ) -> impl Future<Output = RepoResult<Vec<UserPhoneNumber>>>;

    fn find_user_password_by_user_id(
        &self,
        user_id: i64,
    ) -> impl Future<Output = RepoResult<Option<UserPassword>>>;

    fn create_user_with_email_and_password(
        &self,
        name: &str,
        email: &str,
        password: &str,
    ) -> impl Future<Output = RepoResult<User>>;
}
