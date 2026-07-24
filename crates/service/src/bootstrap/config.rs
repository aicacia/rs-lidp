#[cfg(not(feature = "std"))]
use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};

#[derive(Clone, Debug, serde::Deserialize)]
pub struct BootstrapConfig {
    pub lidp_url: String,
    pub lidp_management_url: String,
    pub admin_username: String,
    pub admin_email: String,
    pub admin_password: String,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            lidp_url: "https://lidp.localhost:1337".to_string(),
            lidp_management_url: "https://lidp-management.localhost:1337".to_string(),
            admin_username: "admin".to_string(),
            admin_email: "admin@localhost".to_string(),
            admin_password: "password".to_string(),
        }
    }
}
