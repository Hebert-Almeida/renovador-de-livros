//! Núcleo do Renovador de Livros.
//!
//! Contratos congelados na Onda 0 (ver `docs/PLANO-V2.md`): `model`, traits
//! públicas, `guard::limits` e `store/migrations/0001_init.sql`.

pub mod clock;
pub mod engine;
pub mod error;
pub mod ffi;
pub mod guard;
pub mod keys;
pub mod model;
pub mod planner;
pub mod sophia;
pub mod store;

pub use error::{Error, Result};

/// Versão do app, usada no User-Agent.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Dependências declaradas na Onda 0 e usadas nas ondas seguintes.
use argon2 as _;
use ed25519_dalek as _;
#[cfg(not(target_os = "android"))]
use keyring as _;
use reqwest as _;
use scraper as _;
