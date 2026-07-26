#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(not(feature = "std"))]
extern crate alloc;

mod derived_key;
mod error;
mod master_key;

// Re-exporting types from external crates and internal modules for public use
pub use bip32::{ChildNumber, DerivationPath, Prefix, XPrv};
pub use derived_key::DerivedKey;
pub use error::{KeyError, KeyResult};
pub use master_key::MasterKey;
