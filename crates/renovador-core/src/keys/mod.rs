//! Cofre da chave do banco. Desktop: `keyring` (W1-03). Android: Keystore via
//! plugin (W1-11). iOS: Keychain. Linux sem Secret Service: senha mestra Argon2id.

use crate::store::DbKey;
use crate::{Error, Result};

pub const SERVICE: &str = "br.edu.uftm.editora.renovador";
pub const ENTRY: &str = "db-key";

pub trait KeyStore: Send + Sync {
    /// Devolve a chave existente ou `None` na primeira execução.
    fn load(&self) -> Result<Option<DbKey>>;
    fn store(&self, key: &DbKey) -> Result<()>;
    fn delete(&self) -> Result<()>;
}

/// Gera uma chave nova de 256 bits.
pub fn generate_key() -> DbKey {
    let mut k = DbKey::new([0u8; 32]);
    rand::fill(&mut k[..]);
    k
}

/// Carrega a chave ou cria uma nova e a guarda.
pub fn load_or_create(ks: &dyn KeyStore) -> Result<DbKey> {
    if let Some(k) = ks.load()? {
        return Ok(k);
    }
    let k = generate_key();
    ks.store(&k)?;
    Ok(k)
}

/// Cofre do sistema operacional (W1-03).
#[derive(Debug, Default)]
pub struct OsKeyStore;

impl KeyStore for OsKeyStore {
    fn load(&self) -> Result<Option<DbKey>> {
        Err(Error::todo("keys::OsKeyStore::load"))
    }
    fn store(&self, _key: &DbKey) -> Result<()> {
        Err(Error::todo("keys::OsKeyStore::store"))
    }
    fn delete(&self) -> Result<()> {
        Err(Error::todo("keys::OsKeyStore::delete"))
    }
}

/// Fallback com senha mestra (Argon2id) para Linux sem Secret Service (W1-03).
pub fn derive_from_passphrase(_passphrase: &str, _salt: &[u8; 16]) -> Result<DbKey> {
    Err(Error::todo("keys::derive_from_passphrase"))
}
