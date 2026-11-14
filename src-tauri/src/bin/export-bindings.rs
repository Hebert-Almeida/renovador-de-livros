//! Gera `src/lib/ipc/bindings.ts`: `cargo run -p renovador --bin export-bindings`.
fn main() {
    renovador_lib::export_bindings();
    println!("bindings exportados em {}", renovador_lib::BINDINGS_PATH);
}
