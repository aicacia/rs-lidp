#[derive(Clone)]
pub struct RouterState {
    pub api_base_uri: String,
}

impl RouterState {
    pub fn new(api_base_uri: impl Into<String>) -> Self {
        Self {
            api_base_uri: api_base_uri.into(),
        }
    }
}
