#[cfg(not(feature = "std"))]
use alloc::{format, vec::Vec};

use core::{fmt, str::FromStr};

use bip32::{ChildNumber, DerivationPath, XPrv};
use k256::ecdsa::VerifyingKey as ECDSAVerifyingKey;

use crate::{KeyError, KeyResult};

#[derive(Debug, Clone)]
pub struct DerivedKey {
    key: XPrv,
    derivation_path: DerivationPath,
}

impl fmt::Display for DerivedKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.derivation_path)
    }
}

impl DerivedKey {
    pub fn new(key: XPrv, derivation_path: DerivationPath) -> Self {
        Self {
            key,
            derivation_path,
        }
    }

    pub fn key(&self) -> &XPrv {
        &self.key
    }

    /// Returns the x, y
    pub fn to_xy(&self) -> KeyResult<(Vec<u8>, Vec<u8>)> {
        let extended_public_key = self.key().public_key();
        let public_key = extended_public_key.public_key();
        let encoded_point = public_key.to_encoded_point(false);
        let x_bytes = encoded_point
            .x()
            .ok_or_else(|| KeyError::Other("invalid encoded point x value".into()))?;
        let x = x_bytes.to_vec();
        let y_bytes = encoded_point
            .y()
            .ok_or_else(|| KeyError::Other("invalid encoded point x value".into()))?;
        let y = y_bytes.to_vec();

        Ok((x, y))
    }

    /// Returns the x, y, and d values of the derived key as a tuple of byte vectors.
    pub fn to_xyd(&self) -> KeyResult<(Vec<u8>, Vec<u8>, Vec<u8>)> {
        let private_key = self.key().private_key();
        let d_bytes = private_key.to_bytes();
        let d = d_bytes.to_vec();
        let (x, y) = self.to_xy()?;
        Ok((x, y, d))
    }

    pub fn derivation_path(&self) -> &DerivationPath {
        &self.derivation_path
    }

    pub fn derive_from_child_number(&self, child_number: ChildNumber) -> KeyResult<Self> {
        let key = self.key.derive_child(child_number)?;

        let mut derivation_path = self.derivation_path.clone();
        derivation_path.push(child_number);

        Ok(Self {
            key,
            derivation_path,
        })
    }

    pub fn derive_from_index_and_hardened(&self, index: u32, hardened: bool) -> KeyResult<Self> {
        self.derive_from_child_number(ChildNumber::new(index, hardened)?)
    }

    pub fn derive<S>(&self, path: S) -> KeyResult<Self>
    where
        S: AsRef<str>,
    {
        let path_str = path.as_ref();

        if path_str.starts_with("m") {
            return Err(KeyError::invalid_derivation(format!(
                "Derivation path should not start with 'm': {}",
                path_str
            )));
        }
        let children = path_str
            .split('/')
            .map(str::parse)
            .collect::<Result<Vec<ChildNumber>, bip32::Error>>()?;

        let mut key = self.key.clone();
        for child in &children {
            key = key.derive_child(*child)?;
        }

        let mut derivation_path = self.derivation_path.clone();
        derivation_path.extend(children);

        Ok(Self {
            key,
            derivation_path,
        })
    }

    pub fn verify_derived_key(&self, child: DerivedKey) -> KeyResult<bool> {
        let parent_path = self.derivation_path().as_ref();
        let child_path = child.derivation_path().as_ref();

        if child_path.len() < parent_path.len() {
            return Ok(false);
        }

        if !child_path.starts_with(parent_path) {
            return Ok(false);
        }

        let mut key = self.key.clone();

        for child_number in &child_path[parent_path.len()..] {
            key = key.derive_child(*child_number)?;
        }

        Ok(key.private_key() == child.key().private_key())
    }

    pub fn verify_ecdsa_key<S>(&self, path: S, child: &ECDSAVerifyingKey) -> KeyResult<bool>
    where
        S: AsRef<str>,
    {
        let parent_path = self.derivation_path().as_ref();
        let child_path = DerivationPath::from_str(path.as_ref())?;

        if child_path.len() <= parent_path.len() {
            return Ok(false);
        }

        if !child_path.as_ref().starts_with(parent_path) {
            return Ok(false);
        }

        let mut xpub = self.key.public_key();

        for child in &child_path.as_ref()[parent_path.len()..] {
            if child.is_hardened() {
                return Ok(false);
            }

            match xpub.derive_child(*child) {
                Ok(next) => xpub = next,
                Err(_) => return Ok(false),
            }
        }

        Ok(xpub.public_key() == child)
    }
}
