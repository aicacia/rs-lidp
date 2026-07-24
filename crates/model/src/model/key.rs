#[cfg(not(feature = "std"))]
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use base64::{Engine, engine::general_purpose::STANDARD_NO_PAD};
use chrono::{DateTime, Utc};
use key::{DerivedKey, KeyResult, MasterKey};

use crate::contract::{
    EntityType, JwkPrivate, JwkPrivateParameters, JwkPublic, JwkPublicParameters, JwsAlgorithm,
    KeyUse,
};
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Key {
    pub id: i64,

    #[serde(with = "super::sql_enum::entity_type")]
    pub entity_type: EntityType,
    pub entity_id: i64,
    pub version: i64,

    pub derivation_path: String,
    pub name: String,
    pub hardened: bool,

    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub revoked_at: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub expires_at: Option<DateTime<Utc>>,

    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub updated_at: DateTime<Utc>,
}

impl Key {
    pub fn to_jwk_private(&self, master_key: &MasterKey) -> KeyResult<JwkPrivate> {
        let derived_key = master_key.derive(&self.derivation_path)?;
        let (x, y, d) = derived_key.to_xyd()?;

        let jwt = JwkPrivate {
            r#use: KeyUse::Signature,
            kid: self.id,
            alg: JwsAlgorithm::EdDSA,
            params: JwkPrivateParameters::Ec {
                crv: "secp256k1".to_string(),
                x: STANDARD_NO_PAD.encode(x),
                y: STANDARD_NO_PAD.encode(y),
                d: STANDARD_NO_PAD.encode(d),
            },
        };

        Ok(jwt)
    }

    pub fn to_jwk_public(&self, derived_key: &DerivedKey) -> KeyResult<JwkPublic> {
        let (x, y) = derived_key.to_xy()?;

        let jwt = JwkPublic {
            r#use: KeyUse::Signature,
            kid: self.id,
            alg: JwsAlgorithm::EdDSA,
            params: JwkPublicParameters::Ec {
                crv: "secp256k1".to_string(),
                x: STANDARD_NO_PAD.encode(x),
                y: STANDARD_NO_PAD.encode(y),
            },
        };

        Ok(jwt)
    }

    pub fn build_derivation_path(
        entity_type: EntityType,
        entity_id: i64,
        version: i64,
        hardened: bool,
    ) -> String {
        let mut path = String::new();

        path.push_str("m/1581'/");

        let (entity_id_low, entity_id_high) = split_i64(entity_id);
        let (version_low, version_high) = split_i64(version);

        if hardened {
            path.push_str(&format!(
                "{}'/{}'/{}'/{}'/{}'",
                entity_type as u32, entity_id_low, entity_id_high, version_low, version_high
            ));
        } else {
            path.push_str(&format!(
                "{}/{}/{}/{}/{}",
                entity_type as u32, entity_id_low, entity_id_high, version_low, version_high
            ));
        }

        path
    }

    pub fn parse_derivation_path(path: &str) -> Option<(EntityType, i64, i64, bool)> {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() != 6 {
            return None;
        }

        let hardened = parts[1].ends_with('\'');

        let key_type = match parts[2].trim_end_matches('\'').parse::<u32>() {
            Ok(0) => EntityType::User,
            Ok(1) => EntityType::Client,
            _ => return None,
        };

        let entity_id_low = match parts[3].trim_end_matches('\'').parse::<u32>() {
            Ok(val) => val,
            Err(_) => return None,
        };
        let entity_id_high = match parts[4].trim_end_matches('\'').parse::<u32>() {
            Ok(val) => val,
            Err(_) => return None,
        };
        let entity_id = combine_u32(entity_id_low, entity_id_high);

        let version_low = match parts[5].trim_end_matches('\'').parse::<u32>() {
            Ok(val) => val,
            Err(_) => return None,
        };
        let version_high = match parts[6].trim_end_matches('\'').parse::<u32>() {
            Ok(val) => val,
            Err(_) => return None,
        };
        let version = combine_u32(version_low, version_high);

        Some((key_type, entity_id, version, hardened))
    }
}

fn split_i64(value: i64) -> (u32, u32) {
    let bits = value as u64;
    let low = bits as u32;
    let high = (bits >> 32) as u32;
    (low, high)
}

fn combine_u32(low: u32, high: u32) -> i64 {
    let bits = ((high as u64) << 32) | (low as u64);
    bits as i64
}
