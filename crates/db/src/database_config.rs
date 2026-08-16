use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    pub url: String,
    pub auth_token: Option<String>,
    pub min_connections: u32,
    pub max_connections: u32,
    pub connect_timeout: u64,
    pub acquire_timeout: u64,
    pub idle_timeout: u64,
    pub max_lifetime: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: std::env::var("DATABASE_URL").unwrap_or(":memory:".to_owned()),
            auth_token: std::env::var("DATABASE_AUTH_TOKEN").ok(),
            min_connections: 1,
            max_connections: 100,
            connect_timeout: 10,
            acquire_timeout: 10,
            idle_timeout: 10,
            max_lifetime: 300,
        }
    }
}
