use model::{
    contract::EntityType,
    model::{Key, User},
};

pub trait Principal: Send + Sync {
    fn get_entity_id(&self) -> i64;
    fn get_entity_type(&self) -> EntityType;
    fn get_key(&self) -> &Key;
}

pub struct UserPrincipal {
    pub user: User,
    pub key: Key,
}

impl Principal for UserPrincipal {
    fn get_entity_id(&self) -> i64 {
        self.user.id
    }

    fn get_entity_type(&self) -> EntityType {
        EntityType::User
    }

    fn get_key(&self) -> &Key {
        &self.key
    }
}
