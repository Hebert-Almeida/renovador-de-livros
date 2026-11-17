fn main() {
    // O tauri-build só embute o manifesto do Windows (Common Controls v6) nos
    // `[[bin]]`. Examples e testes que linkam o Tauri morrem então com
    // STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139). Aqui o próprio linker embute o
    // mesmo manifesto em todos os alvos, como indica a documentação de
    // `WindowsAttributes::new_without_app_manifest`.
    let mut attrs = tauri_build::Attributes::new();
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        attrs =
            attrs.windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
        let dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
        let manifest = std::path::Path::new(&dir).join("windows-app-manifest.xml");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    }
    tauri_build::try_build(attrs).expect("tauri-build falhou");
}
