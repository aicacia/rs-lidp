#[cfg(not(feature = "std"))]
use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
    vec,
    vec::Vec,
};
#[cfg(feature = "std")]
use std::sync::Arc;

use model::{
    contract::{
        ClientProfile, ClientRegistration, ClientType, EntityType, GrantType, ResponseType,
        TokenEndpointAuthMethod,
    },
    model::{Application, Client, Key, User},
};

use super::config::BootstrapConfig;
use crate::{
    repo::{
        ApplicationRepo, ClientRepo, KeyRepo, KeyService, PrivateKeyRepo, RepoResult, UserRepo,
    },
    util::generate_random_string,
};

pub struct BootstrapService<A, C, K, U> {
    application_repo: A,
    client_repo: C,
    user_repo: U,
    key_service: Arc<KeyService<K>>,
    config: BootstrapConfig,
}

impl<A, C, K, U> BootstrapService<A, C, K, U>
where
    A: ApplicationRepo,
    C: ClientRepo,
    K: KeyRepo,
    U: UserRepo,
{
    pub fn new(
        application_repo: A,
        client_repo: C,
        user_repo: U,
        key_service: Arc<KeyService<K>>,
        config: BootstrapConfig,
        _key_namespace: impl Into<String>,
    ) -> Self {
        Self {
            application_repo,
            client_repo,
            user_repo,
            key_service,
            config,
        }
    }

    pub async fn ensure_system_baseline(&self) -> RepoResult<()> {
        let lidp_application = self
            .ensure_application("Local IdP".to_string(), "lidp".to_string())
            .await?;

        let lidp_web_client = self
            .ensure_client(
                lidp_application.id,
                "lidp-web".to_string(),
                "Local IdP Web".to_string(),
                self.config.lidp_url.clone(),
                ClientProfile::Web,
            )
            .await?;
        let _lidp_web_client_key = self
            .ensure_active_key(
                EntityType::Client,
                lidp_web_client.id,
                "Local IdP Web",
                lidp_web_client.client_secret.as_str(),
                true,
            )
            .await?;

        let lidp_desktop_client = self
            .ensure_client(
                lidp_application.id,
                "lidp-desktop".to_string(),
                "Local IdP Desktop".to_string(),
                "lidp://app".to_string(),
                ClientProfile::Native,
            )
            .await?;
        let _lidp_desktop_client_key = self
            .ensure_active_key(
                EntityType::Client,
                lidp_desktop_client.id,
                "Local IdP Desktop",
                lidp_desktop_client.client_secret.as_str(),
                true,
            )
            .await?;

        let lidp_management_application = self
            .ensure_application(
                "Local IdP Management".to_string(),
                "lidp-management".to_string(),
            )
            .await?;

        let lidp_management_web_client = self
            .ensure_client(
                lidp_management_application.id,
                "lidp-management-web".to_string(),
                "Local IdP Management Web".to_string(),
                self.config.lidp_management_url.clone(),
                ClientProfile::Web,
            )
            .await?;
        let _idp_management_web_client_key = self
            .ensure_active_key(
                EntityType::Client,
                lidp_management_web_client.id,
                "Local IdP Management Web",
                lidp_management_web_client.client_secret.as_str(),
                true,
            )
            .await?;

        let lidp_management_desktop_client = self
            .ensure_client(
                lidp_management_application.id,
                "lidp-management-desktop".to_string(),
                "Local IdP Management Desktop".to_string(),
                "lidp-management://app".to_string(),
                ClientProfile::Native,
            )
            .await?;
        let _lidp_management_desktop_client_key = self
            .ensure_active_key(
                EntityType::Client,
                lidp_management_desktop_client.id,
                "Local IdP Management Desktop",
                lidp_management_desktop_client.client_secret.as_str(),
                true,
            )
            .await?;

        let admin_user = self.ensure_admin_user().await?;
        let _admin_user_key = self
            .ensure_active_key(
                EntityType::User,
                admin_user.id,
                &self.config.admin_username,
                &self.config.admin_password,
                true,
            )
            .await?;

        Ok(())
    }

    async fn ensure_application(&self, name: String, uri: String) -> RepoResult<Application> {
        if let Some(application) = self.application_repo.find_by_uri(&uri).await? {
            log::debug!("Found existing application with name: {}", uri);
            return Ok(application);
        }

        let application = self
            .application_repo
            .create_application(name, uri, None)
            .await?;

        log::debug!("Created application with name: {}", application.name);
        Ok(application)
    }

    async fn ensure_client(
        &self,
        application_id: i64,
        client_id: String,
        client_name: String,
        client_uri: String,
        profile: ClientProfile,
    ) -> RepoResult<Client> {
        let expected_scopes = vec![
            "openid".to_owned(),
            "profile".to_owned(),
            "address".to_owned(),
            "offline".to_owned(),
            "email".to_owned(),
            "phone".to_owned(),
        ];
        let existing = self
            .client_repo
            .find_client_by_client_id(&client_id)
            .await?;
        let redirect_uris = vec![format!("{}/callback", client_uri)];

        if let Some(mut client) = existing {
            let mut changed = false;

            if client.application_id != application_id {
                client.application_id = application_id;
                changed = true;
            }

            if client.client_name != client_name {
                client.client_name = client_name.to_string();
                changed = true;
            }

            if client.client_uri != client_uri {
                client.client_uri = client_uri.to_string();
                changed = true;
            }

            if client.allowed_scopes != expected_scopes {
                client.allowed_scopes = expected_scopes;
                changed = true;
            }

            if client.profile != profile {
                client.profile = profile;
                changed = true;
            }

            if client.redirect_uris != redirect_uris {
                client.redirect_uris = redirect_uris;
                changed = true;
            }

            if changed {
                log::debug!("Updating client with client_id: {}", client.client_id);
                let client = self.client_repo.update_client(client).await?;
                log::debug!("Updated client with client_id: {}", client.client_id);
                Ok(client)
            } else {
                Ok(client)
            }
        } else {
            let client = ClientRegistration {
                application_id,
                client_id: Some(client_id),
                client_secret: Some(generate_random_string::<32>()),
                client_id_issued_at: None,
                client_secret_expires_at: None,
                client_name,
                client_uri: Some(client_uri),
                redirect_uris,
                client_type: ClientType::Public,
                profile,
                token_endpoint_auth_method: TokenEndpointAuthMethod::None,
                allowed_grant_types: vec![
                    GrantType::Password,
                    GrantType::ClientCredentials,
                    GrantType::AuthorizationCode,
                    GrantType::RefreshToken,
                ],
                response_types: vec![ResponseType::Code],
                allowed_scopes: expected_scopes,
                logo_uri: None,
                contacts: Vec::new(),
                terms_of_service_uri: None,
                policy_uri: None,
                software_statement: None,
                software_id: None,
                software_version: None,
            };

            log::debug!("Creating new client with client_id: {:?}", client.client_id);
            let client = self.client_repo.create_client(client).await?;
            log::debug!("Created client with client_id: {}", client.client_id);
            Ok(client)
        }
    }

    async fn ensure_admin_user(&self) -> RepoResult<User> {
        if let Some(user) = self
            .user_repo
            .find_user_by_username_or_email(&self.config.admin_email)
            .await?
        {
            log::debug!(
                "Found existing admin user with email: {}",
                self.config.admin_email
            );
            return Ok(user);
        }

        let user = self
            .user_repo
            .create_user_with_email_and_password(
                &self.config.admin_username,
                &self.config.admin_email,
                &self.config.admin_password,
            )
            .await?;

        log::debug!("Created admin user with email: {}", self.config.admin_email);
        Ok(user)
    }

    async fn ensure_active_key(
        &self,
        entity_type: EntityType,
        entity_id: i64,
        name: &str,
        passphrase: &str,
        hardened: bool,
    ) -> RepoResult<Key> {
        let scoped_namespace = self.key_service.scoped_namespace(entity_type, entity_id);

        self.key_service
            .ensure_entity_master_key(entity_type, entity_id, passphrase)?;

        if let Some(key) = self
            .key_service
            .key_repo()
            .find_active_entity_root_key(entity_type, entity_id)
            .await?
        {
            self.key_service
                .private_key_repo()
                .ensure_derivation_path(&scoped_namespace, key.derivation_path()?)?;
            return Ok(key);
        }

        let (key, _derived_key) = self
            .key_service
            .create_key(
                None,
                entity_type,
                entity_id,
                hardened,
                name.to_owned(),
                None,
            )
            .await?;

        log::debug!("Created new active key for entity_id: {}", entity_id);
        Ok(key)
    }
}
