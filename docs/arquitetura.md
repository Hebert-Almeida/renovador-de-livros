# Arquitetura

Visão geral do Renovador de Livros 2.0. O plano completo está em [PLANO-V2.md](PLANO-V2.md).

## Camadas

```
SvelteKit (SPA, adapter-static)  ── src/
        │  IPC tipado (tauri-specta → src/lib/ipc/bindings.ts)
Tauri 2 (src-tauri/)  ── comandos, bandeja, agendador, notificações
        │
renovador-core (crates/renovador-core/)  ── núcleo em Rust, sem dependência do Tauri
        │  FFI (ffi::run_background_cycle)
Plugin mobile (plugins/tauri-plugin-renovador-bg/)  ── WorkManager (Android), BGTaskScheduler (iOS)
```

## Núcleo (`crates/renovador-core/src`)

| Módulo | Responsabilidade |
|---|---|
| `model.rs` | Tipos de domínio (serde + specta) |
| `clock.rs` | `Clock`, `SystemClock`, `FakeClock` (testes determinísticos) |
| `guard/` | `RequestGate`: semáforo, espaçamento, cotas, breaker, validação de host e política remota. Constantes fixas em `guard/limits.rs` |
| `store/` | Banco SQLCipher, migrações, repositórios, backup e "apagar tudo" |
| `keys/` | `KeyStore`: chave do banco no cofre do sistema (ou senha mestra Argon2id) |
| `sophia/` | Cliente HTTP do Terminal Web Sophia, detecção de CAPTCHA/Cloudflare e parser do HTML |
| `planner/` | Regras de agendamento como funções puras (fuso America/Sao_Paulo) |
| `engine/` | `run_cycle`: política → login → lista → sincroniza → planeja → renova e confirma → registra → logout |
| `ffi.rs` | Ponto de entrada do ciclo em segundo plano no mobile |

## Motor HTTP

Sem navegador embutido. O cliente faz login em `{base}/mobile/ajx_efetua_login.asp`, lê a lista de empréstimos, segue o link de renovação de cada livro na janela e **faz um novo GET da lista para confirmar a nova data**. Cada requisição passa pelo `RequestGate`. Detalhes na seção 5.1 do plano e em [uso responsável](legal/uso-responsavel.md).

## Dados

Banco local SQLCipher (tabelas `account`, `book`, `loan`, `renewal_event`, `schedule`, `request_log`, `breaker_state`, `setting`). A senha da conta é opcional (`NULL` = modo "sem salvar senha"). Ver [aviso de privacidade](legal/aviso-de-privacidade.md).

## Segundo plano

| Plataforma | Mecanismo |
|---|---|
| Desktop | Bandeja + loop tokio a cada 30 min, com catch-up ao iniciar |
| Android | WorkManager (periódico de 6 h + execução única) → JNI → núcleo |
| iOS | BGAppRefreshTask (melhor esforço) + notificação "toque para renovar" |

## Política remota

`policy.json` + `policy.json.sig` (Ed25519, chave pública embutida), consultada no máximo 1×/dia. O limite efetivo é o mínimo entre o embutido e o remoto; a política só aperta. Operação no [guia de manutenção](guia-de-manutencao.md).
