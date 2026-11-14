// Ponto único de acesso ao IPC tipado. `bindings.ts` é gerado pelo tauri-specta
// (cargo test -p renovador -- export_bindings) e NÃO deve ser editado à mão.
export * from './bindings.ts';
export { commands, events } from './bindings.ts';
