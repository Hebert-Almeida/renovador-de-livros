//! Testes do cofre de chaves com backend em memória (nunca toca o cofre real).

use renovador_core::keys::{
    BackendKeyStore, ENTRY, KeyStore, MemoryBackend, SERVICE, SecretBackend,
    derive_from_passphrase, generate_key, generate_salt, load_or_create,
};

#[test]
fn chaves_geradas_sao_aleatorias() {
    let a = generate_key();
    let b = generate_key();
    assert_ne!(*a, *b);
    assert_ne!(*a, [0u8; 32]);
}

#[test]
fn primeira_execucao_cria_e_depois_reutiliza() {
    let ks = BackendKeyStore::new(MemoryBackend::new());
    assert!(ks.load().unwrap().is_none());
    let k1 = load_or_create(&ks).unwrap();
    let k2 = load_or_create(&ks).unwrap();
    assert_eq!(*k1, *k2);
    assert_eq!(*ks.load().unwrap().unwrap(), *k1);
    let raw = ks.backend().get(SERVICE, ENTRY).unwrap().unwrap();
    assert_eq!(raw.as_slice(), &k1[..]);
}

#[test]
fn delete_remove_e_e_idempotente() {
    let ks = BackendKeyStore::new(MemoryBackend::new());
    ks.store(&generate_key()).unwrap();
    ks.delete().unwrap();
    assert!(ks.load().unwrap().is_none());
    ks.delete().unwrap();
}

#[test]
fn tamanho_invalido_e_erro() {
    let ks = BackendKeyStore::new(MemoryBackend::new());
    ks.backend().set(SERVICE, ENTRY, &[1, 2, 3]).unwrap();
    assert!(ks.load().is_err());
}

#[test]
fn argon2id_e_deterministico_e_depende_do_sal() {
    let salt = generate_salt();
    let a = derive_from_passphrase("senha mestra", &salt).unwrap();
    let b = derive_from_passphrase("senha mestra", &salt).unwrap();
    assert_eq!(*a, *b);
    let c = derive_from_passphrase("outra senha", &salt).unwrap();
    assert_ne!(*a, *c);
    let d = derive_from_passphrase("senha mestra", &[0u8; 16]).unwrap();
    assert_ne!(*a, *d);
    assert!(derive_from_passphrase("", &salt).is_err());
}
