//! Ponte JNI usada pelo `RenewWorker`: `NativeBridge.runBackgroundCycle`.

use jni::JNIEnv;
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::jstring;
use renovador_core::store::DbKey;

fn error_json(msg: &str) -> String {
    format!("{{\"error\":{:?}}}", msg)
}

fn run(env: &mut JNIEnv, db_path: &JString, key: &JByteArray) -> String {
    let path: String = match env.get_string(db_path) {
        Ok(s) => s.into(),
        Err(_) => return error_json("caminho do banco inválido"),
    };
    let bytes = match env.convert_byte_array(key) {
        Ok(b) => zeroize::Zeroizing::new(b),
        Err(_) => return error_json("chave inválida"),
    };
    let Ok(arr) = <[u8; 32]>::try_from(bytes.as_slice()) else {
        return error_json("a chave deve ter 32 bytes");
    };
    let key = DbKey::new(arr);
    std::panic::catch_unwind(|| renovador_core::ffi::run_background_cycle(&path, &key))
        .unwrap_or_else(|_| error_json("pânico no ciclo em segundo plano"))
}

/// `external fun runBackgroundCycle(dbPath: String, key: ByteArray): String`
/// em `app.renovador.bg.NativeBridge`.
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_renovador_bg_NativeBridge_runBackgroundCycle<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    db_path: JString<'l>,
    key: JByteArray<'l>,
) -> jstring {
    let out = run(&mut env, &db_path, &key);
    env.new_string(out)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
