use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    #[default]
    Local,
    Development,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Development => "development",
            Environment::Production => "production",
        }
    }
}
