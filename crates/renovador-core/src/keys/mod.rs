//! Cofre da chave do banco. Desktop: `keyring` (W1-03). Android: Keystore via
//! plugin (W1-11). iOS: Keychain. Linux sem Secret Service: senha mestra Argon2id.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use zeroize::{Zeroize, Zeroizing};

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

/// Segredo binário que é zerado ao sair de escopo.
pub type Secret = Zeroizing<Vec<u8>>;

/// Backend de segredos binários (abstração sobre o cofre do SO, permite mock).
pub trait SecretBackend: Send + Sync {
    /// `Ok(None)` quando não há segredo guardado.
    fn get(&self, service: &str, entry: &str) -> Result<Option<Secret>>;
    fn set(&self, service: &str, entry: &str, secret: &[u8]) -> Result<()>;
    /// Apagar algo inexistente não é erro.
    fn delete(&self, service: &str, entry: &str) -> Result<()>;
}

type MemoryMap = HashMap<(String, String), Secret>;

/// Backend em memória, para testes (nunca toca o cofre real).
#[derive(Debug, Default)]
pub struct MemoryBackend {
    items: Mutex<MemoryMap>,
}

impl MemoryBackend {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> Result<MutexGuard<'_, MemoryMap>> {
        self.items
            .lock()
            .map_err(|_| Error::Keys("mutex envenenado".into()))
    }
}

impl SecretBackend for MemoryBackend {
    fn get(&self, service: &str, entry: &str) -> Result<Option<Secret>> {
        Ok(self
            .lock()?
            .get(&(service.to_owned(), entry.to_owned()))
            .cloned())
    }
    fn set(&self, service: &str, entry: &str, secret: &[u8]) -> Result<()> {
        self.lock()?.insert(
            (service.to_owned(), entry.to_owned()),
            Zeroizing::new(secret.to_vec()),
        );
        Ok(())
    }
    fn delete(&self, service: &str, entry: &str) -> Result<()> {
        self.lock()?.remove(&(service.to_owned(), entry.to_owned()));
        Ok(())
    }
}

/// Backend do cofre do sistema (Credential Manager, Keychain, Secret Service).
#[derive(Debug, Default)]
pub struct KeyringBackend;

#[cfg(not(target_os = "android"))]
fn keyring_err(e: keyring::Error) -> Error {
    Error::Keys(e.to_string())
}

#[cfg(not(target_os = "android"))]
impl SecretBackend for KeyringBackend {
    fn get(&self, service: &str, entry: &str) -> Result<Option<Secret>> {
        let e = keyring::Entry::new(service, entry).map_err(keyring_err)?;
        match e.get_secret() {
            Ok(s) => Ok(Some(Zeroizing::new(s))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(keyring_err(err)),
        }
    }
    fn set(&self, service: &str, entry: &str, secret: &[u8]) -> Result<()> {
        keyring::Entry::new(service, entry)
            .and_then(|e| e.set_secret(secret))
            .map_err(keyring_err)
    }
    fn delete(&self, service: &str, entry: &str) -> Result<()> {
        let e = keyring::Entry::new(service, entry).map_err(keyring_err)?;
        match e.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(keyring_err(err)),
        }
    }
}

#[cfg(target_os = "android")]
fn android_unavailable() -> Error {
    Error::Keys("keyring indisponível no Android (use o plugin)".into())
}

#[cfg(target_os = "android")]
impl SecretBackend for KeyringBackend {
    fn get(&self, _: &str, _: &str) -> Result<Option<Secret>> {
        Err(android_unavailable())
    }
    fn set(&self, _: &str, _: &str, _: &[u8]) -> Result<()> {
        Err(android_unavailable())
    }
    fn delete(&self, _: &str, _: &str) -> Result<()> {
        Err(android_unavailable())
    }
}

/// `KeyStore` sobre qualquer `SecretBackend`, na entrada `SERVICE`/`ENTRY`.
#[derive(Debug, Default)]
pub struct BackendKeyStore<B> {
    backend: B,
}

impl<B: SecretBackend> BackendKeyStore<B> {
    pub fn new(backend: B) -> Self {
        Self { backend }
    }
    pub fn backend(&self) -> &B {
        &self.backend
    }
}

impl<B: SecretBackend> KeyStore for BackendKeyStore<B> {
    fn load(&self) -> Result<Option<DbKey>> {
        let Some(secret) = self.backend.get(SERVICE, ENTRY)? else {
            return Ok(None);
        };
        if secret.len() != 32 {
            return Err(Error::Keys(format!(
                "chave guardada com tamanho inválido ({} bytes)",
                secret.len()
            )));
        }
        let mut key = DbKey::new([0u8; 32]);
        key.copy_from_slice(&secret);
        Ok(Some(key))
    }
    fn store(&self, key: &DbKey) -> Result<()> {
        self.backend.set(SERVICE, ENTRY, &key[..])
    }
    fn delete(&self) -> Result<()> {
        self.backend.delete(SERVICE, ENTRY)
    }
}

/// Cofre do sistema operacional.
#[derive(Debug, Default)]
pub struct OsKeyStore;

impl KeyStore for OsKeyStore {
    fn load(&self) -> Result<Option<DbKey>> {
        BackendKeyStore::new(KeyringBackend).load()
    }
    fn store(&self, key: &DbKey) -> Result<()> {
        BackendKeyStore::new(KeyringBackend).store(key)
    }
    fn delete(&self) -> Result<()> {
        BackendKeyStore::new(KeyringBackend).delete()
    }
}

/// Gera um sal aleatório de 16 bytes para `derive_from_passphrase`.
pub fn generate_salt() -> [u8; 16] {
    let mut s = [0u8; 16];
    rand::fill(&mut s[..]);
    s
}

/// Fallback com senha mestra (Argon2id v1.3, parâmetros padrão) para Linux sem
/// Secret Service. O sal não é secreto e deve ser persistido junto ao banco.
pub fn derive_from_passphrase(passphrase: &str, salt: &[u8; 16]) -> Result<DbKey> {
    if passphrase.is_empty() {
        return Err(Error::Invalid("senha mestra vazia".into()));
    }
    let mut out = DbKey::new([0u8; 32]);
    let argon = argon2::Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::default(),
    );
    if let Err(e) = argon.hash_password_into(passphrase.as_bytes(), salt, &mut out[..]) {
        out.zeroize();
        return Err(Error::Keys(format!("falha no Argon2id: {e}")));
    }
    Ok(out)
}
