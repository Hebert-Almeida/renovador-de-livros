# Renovador de Livros 2.0 — Plano de execução em ondas (`/batch`)

## Como executar

O `/batch` roda unidades **independentes** em paralelo: cada uma num worktree, cada uma vira um PR. Uma reescrita do zero tem dependências fortes, então o trabalho está dividido em ondas. **Só comece uma onda depois que a anterior estiver mergeada em `main`.**

1. **Onda 0**, sequencial, numa sessão normal: `Execute a Onda 0 de docs/PLANO-V2.md`. Ela congela os contratos (tipos, comandos IPC, schema, dependências e rotas). Depois, commit e push em `main`.
2. **Onda 1** (11 unidades): `/batch Execute as unidades da Onda 1 de docs/PLANO-V2.md`. Revisar e mergear os PRs.
3. **Onda 2** (12 unidades): `/batch Execute as unidades da Onda 2 de docs/PLANO-V2.md`. Revisar e mergear os PRs.
4. **Onda 3**, sessão normal: integração mobile, empacotamento, QA e entrega. Correções vindas do QA podem virar um novo `/batch`.

## 1. Contexto

Este projeto será entregue à **Editora da UFTM**. Hoje o app (Electron 40 + Playwright) abre o Chrome/Edge em modo headless, faz login no Terminal Web Sophia da UFTM e abre os links de renovação. Os problemas são:

- só roda em desktop;
- conta como renovado sem verificar se a renovação deu certo (`main.js:171-226`);
- esconde que é automação (`--disable-blink-features=AutomationControlled` + User-Agent falso, `main.js:100-106`);
- não tem histórico, agenda nem notificações;
- o README diz "não armazena credenciais", o que deixará de ser verdade.

A versão 2.0 é um automatizador completo e multiplataforma:
- Windows, Linux e Android obrigatórios; macOS e iOS desejáveis;
- banco local criptografado;
- renovação e reagendamento automáticos, com notificações;
- CRUD de agendamentos e estatísticas;
- interface neo-brutalista com tema de livro;
- foco em performance e acessibilidade;
- proteções anti-abuso (anti-DDoS).

## 2. Decisões tomadas

| Tema | Decisão |
|---|---|
| Base | **Tauri 2 + núcleo em Rust (`crates/renovador-core`) + SvelteKit (SPA, adapter-static) + TypeScript** |
| Motor | **HTTP puro**, sem navegador embutido (detalhes em 5.1) |
| "Caminho desejado" | **Endereço da biblioteca Sophia**, configurável por conta; UFTM `https://biblioteca.sophia.com.br/5362` pré-configurada |
| Licença | **`MIT OR Apache-2.0`**, saindo da GPL-3.0. Isso permite incluir o GSAP, cuja licença "Standard No Charge" não é OSI |
| Gate jurídico | Desenvolver já; **publicar só após anuência por escrito da Biblioteca/SIBi UFTM**. Se negarem, o projeto para (ver seção 11) |
| Animação | GSAP + anime.js + Three.js (com regras de performance e acessibilidade, em 5.7) |
| Design | skills **impeccable** e **taste-skill** (`design-taste-frontend`) instaladas no projeto |

## 3. Regras para todas as unidades (agentes)

1. **Nunca acessar o site real do Sophia** em testes, CI ou agentes. Use só `wiremock` + fixtures. O acesso real é feito apenas pelo usuário, manualmente (P3 e Onda 3).
2. **Edite somente os arquivos da sua unidade.** Os contratos congelados na Onda 0 não mudam:
   - `model.rs`, traits, `guard/limits.rs`, `store/migrations/0001_init.sql`;
   - assinaturas em `src-tauri/src/commands.rs` e `src/lib/ipc/bindings.ts`;
   - `Cargo.toml`, `package.json`, `+layout.svelte` e a lista de rotas.

   Se uma mudança for indispensável, **pare e explique no PR**.
3. As constantes anti-abuso (5.2) só existem em `guard/limits.rs`. Nunca podem ser ajustadas por arquivo de configuração, CLI ou variável de ambiente.
4. Nada de credenciais, dados pessoais ou HTML não anonimizado no repositório.
5. Identificadores em inglês. Textos da UI, documentação, commits e PRs em pt-BR, com Conventional Commits.
6. Antes do PR, rode o que se aplicar: `cargo fmt --all --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`, `npm run check`, `npm run test`, `npm run build`.
7. Toda unidade de UI precisa de teste axe sem violações e de teste de navegação só por teclado. Animações passam por `src/lib/motion`, que respeita `prefers-reduced-motion`.
8. Three.js só via `import(`C:\Strawberry\perl\bin`)` dinâmico. Não adicione dependências além das declaradas na Onda 0 sem justificar no PR.

## 4. Pré-requisitos manuais (antes da Onda 1)

- **P1 — Toolchain neste PC.**
  - Faltam: rustup (stable-msvc), VS Build Tools 2022 (C++), **Strawberry Perl** (o perl do Git Bash não compila OpenSSL), **JDK 17+** (o instalado é o 8; configurar `JAVA_HOME`) e **Android NDK** (o SDK existe, o NDK não; configurar `NDK_HOME`).
  - Depois: `rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android`.
- **P2 — GitHub.** ✓ O `gh` já está autenticado (conta Hebert-Almeida, escopos `repo` e `workflow`) e o `origin` é `github.com/Hebert-Almeida/renovador-de-livros`.
- **P3 — Fixtures**, capturadas pelo próprio usuário no navegador (necessárias para a W1-06):
  1. Abrir `https://biblioteca.sophia.com.br/5362/mobile/`, ir em Renovações e fazer login.
  2. Salvar a lista de empréstimos (Ctrl+S, "somente HTML") como `lista-emprestimos.html`. Se possível, salvar também uma lista vazia como `lista-vazia.html`.
  3. Em DevTools › Rede, copiar a resposta de `ajx_efetua_login.asp` com login certo e com senha errada: `login-ok.json` e `login-erro.json`.
  4. Na próxima renovação real, salvar a página de resposta como `renovacao-sucesso.html`. Se houver recusa (reserva, limite), salvar como `renovacao-recusa-<motivo>.html`.
  5. Anonimizar nome, login/matrícula, e-mail e códigos pessoais, mantendo o formato, e salvar tudo em `crates/renovador-core/tests/fixtures/`.

## 5. Especificação de referência

### 5.1 Motor HTTP Sophia (`{base}` = endereço da biblioteca)

1. `GET {base}/mobile/login.asp?idioma=ptbr&acesso=web&destino=6&detalhe=0`. A resposta define os cookies `TESTE` e `ASPSESSIONID…`.
2. `POST {base}/mobile/ajx_efetua_login.asp`, form-urlencoded, com corpo `idioma=ptbr&codigo=<login>&senha=<senha>&usuarioexterno=0&detalhe=0&acesso=web&destino=6&midia=0&build_app=&mobile=0`.
   - Resposta JSON: `{login_valido:"true"|"false", mensagem, url, abrir_popup}`.
   - O `url` vem entre aspas; o site remove um caractere de cada ponta (`substring(1, len-1)`).
3. `GET {base}/mobile/<url>` → lista de empréstimos. Seletores conhecidos:
   - título: `div.ui-content > ul > li > div.div-textoLista > h3`;
   - link de renovação (GET): `div.ui-content > ul > li > div.div-textoLista > div.ui-grid-a > div.ui-block-b > a`.
4. Para cada livro dentro da janela: `GET <link>`, classificar a resposta e **fazer um novo GET da lista para confirmar a nova data**.
5. Logout, se houver endpoint.

Regras do cliente:
- O site usa UTF-8, passa pelo Cloudflare e hoje não tem CSRF nem CAPTCHA.
- **Abortar sem nova tentativa** quando aparecer:
  - CAPTCHA (`g-recaptcha`, `h-captcha`, `cf-turnstile`, "captcha");
  - desafio do Cloudflare ("Just a moment", `cf-chl`);
  - host que não seja Sophia;
  - HTML com estrutura inesperada.
- Cabeçalhos: `User-Agent: RenovadorLivrosUFTM/<versão> (+https://github.com/Hebert-Almeida/renovador-de-livros)`, `Accept-Language: pt-BR`, gzip.
- Nunca baixar imagens, CSS ou JS. Timeout de 15 s.

### 5.2 Anti-abuso (`guard/limits.rs`)

| Regra | Valor |
|---|---|
| Requisições simultâneas | 1 (global) |
| Intervalo entre requisições | ≥ 2 s + jitter de 0–1 s |
| Máx. requisições por sessão | 20 |
| Sessões automáticas por conta/dia | ≤ 2 |
| Sessões manuais por conta/dia | ≤ 4, com espera ≥ 30 min entre elas |
| Sessões por instalação/dia | ≤ 10 |
| Contas por instalação | ≤ 3 (padrão: 1) |
| Logins distintos testados por instalação/dia | ≤ 3; senha errada pausa a conta até o usuário editar a senha |
| Falhas consecutivas (rede/5xx/HTML inesperado) | 3 → pausa de 6 h; 5 → pausa de 24 h + notificação |
| HTTP 429/503 | respeita `Retry-After`, mínimo 1 h |
| CAPTCHA, desafio do Cloudflare ou bloqueio | para na hora, sem retry; pede ação manual e abre o site oficial |
| Endereço da biblioteca | só HTTPS e precisa se identificar como Sophia (cabeçalho `x-powered-by: Sophia` + página de login reconhecida); cotas aplicadas também por host |
| Horário automático | 07h–21h, com minuto aleatório fixo por instalação |
| Política remota (kill switch) | `policy.json` + `policy.json.sig` (Ed25519, chave pública embutida), consultada no máximo 1×/dia |

Persistência e política remota:
- Tudo fica persistido em `request_log` e `breaker_state`: reiniciar o app não zera as cotas.
- O plugin single-instance impede execuções paralelas.
- Política remota:
  - cada limite efetivo é o **mínimo** entre o embutido e o remoto;
  - `automation_enabled` vale `embutido E remoto`;
  - `blocked_hosts` é a união das duas listas.

  Ou seja, **a política remota só aperta, nunca afrouxa**.

### 5.3 Banco SQLCipher (`store/migrations/0001_init.sql`)

```sql
CREATE TABLE account (id INTEGER PRIMARY KEY, library_base_url TEXT NOT NULL, library_name TEXT NOT NULL,
  login TEXT NOT NULL, password TEXT /* NULL = modo "sem salvar senha" */, mode TEXT NOT NULL CHECK (mode IN ('auto','manual')),
  status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','paused','invalid_password','blocked')),
  renew_days_before INTEGER NOT NULL DEFAULT 1 CHECK (renew_days_before BETWEEN 0 AND 3),
  remind_days_before INTEGER NOT NULL DEFAULT 2 CHECK (remind_days_before BETWEEN 0 AND 7),
  last_sync_at TEXT, last_renewal_at TEXT, next_renewal_at TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
  UNIQUE (library_base_url, login));
CREATE TABLE book (id INTEGER PRIMARY KEY, account_id INTEGER NOT NULL REFERENCES account(id) ON DELETE CASCADE,
  item_key TEXT NOT NULL, title TEXT NOT NULL, author TEXT, call_number TEXT, library_branch TEXT,
  first_seen_at TEXT NOT NULL, UNIQUE (account_id, item_key));
CREATE TABLE loan (id INTEGER PRIMARY KEY, book_id INTEGER NOT NULL REFERENCES book(id) ON DELETE CASCADE,
  loaned_at TEXT, due_date TEXT NOT NULL, renewals_count INTEGER NOT NULL DEFAULT 0, renewals_limit INTEGER,
  status TEXT NOT NULL CHECK (status IN ('active','returned','overdue')), auto_renew INTEGER NOT NULL DEFAULT 1,
  returned_at TEXT, updated_at TEXT NOT NULL);
CREATE TABLE renewal_event (id INTEGER PRIMARY KEY, loan_id INTEGER NOT NULL REFERENCES loan(id) ON DELETE CASCADE,
  attempted_at TEXT NOT NULL, trigger TEXT NOT NULL CHECK (trigger IN ('auto','manual','catch_up')),
  result TEXT NOT NULL CHECK (result IN ('success','reserved','limit_reached','blocked','overdue','network_error','parse_error','captcha','rate_limited')),
  old_due_date TEXT, new_due_date TEXT, message TEXT);
CREATE TABLE schedule (id INTEGER PRIMARY KEY, account_id INTEGER NOT NULL REFERENCES account(id) ON DELETE CASCADE,
  loan_id INTEGER REFERENCES loan(id) ON DELETE CASCADE, kind TEXT NOT NULL CHECK (kind IN ('renew','sync','remind')),
  run_at TEXT NOT NULL, status TEXT NOT NULL CHECK (status IN ('pending','paused','done','cancelled','failed')),
  origin TEXT NOT NULL CHECK (origin IN ('system','user')), updated_at TEXT NOT NULL);
CREATE TABLE request_log (id INTEGER PRIMARY KEY, account_id INTEGER, host TEXT NOT NULL, at TEXT NOT NULL,
  kind TEXT NOT NULL, session_id TEXT NOT NULL, http_status INTEGER, outcome TEXT NOT NULL);
CREATE TABLE breaker_state (scope TEXT PRIMARY KEY, consecutive_failures INTEGER NOT NULL DEFAULT 0, open_until TEXT, reason TEXT);
CREATE TABLE setting (key TEXT PRIMARY KEY, value TEXT NOT NULL);
-- datas: timestamps ISO-8601 UTC; due_date 'YYYY-MM-DD' (America/Sao_Paulo). Índices por FK, status e run_at.
```

Gestão da chave (aleatória, 256 bits):
- **Desktop**: cofre do sistema (crate `keyring`). Linux sem Secret Service usa senha mestra (Argon2id).
- **Android**: Keystore.
- **iOS**: Keychain.

Outros recursos:
- `zeroize` para limpar segredos da memória;
- backup e restauração criptografados;
- "apagar tudo".

### 5.4 Agendamento e notificações

| Plataforma | Mecanismo | Garantia |
|---|---|---|
| Desktop | inicia com o sistema e fica na bandeja; um loop tokio acorda a cada 30 min e só usa a rede se houver tarefa vencida. Ao iniciar, executa o que ficou pendente (catch-up). Fechar a janela libera o WebView | Alta enquanto o PC estiver ligado |
| Android | WorkManager (periódico a cada 6 h + execução única no horário planejado, exigindo rede) → JNI → núcleo | Alta (o Doze pode atrasar) |
| iOS | BGAppRefreshTask (~30 s, oportunista) + notificação "toque para renovar" | Melhor esforço |

Regras do planejador:
- Renova em *vencimento − N dias* (padrão: 1).
- Nova tentativa no dia do vencimento **só** se a falha foi de rede.
- Reserva, limite ou bloqueio: notifica na hora com a data de devolução e não tenta de novo.
- Lembretes em D−2 e D0 para livros sem renovação automática ou cuja renovação falhou.
- Sincronização de descoberta a cada 3 dias.
- Agendamentos criados pelo usuário (`origin=user`) que foram cancelados ou pausados nunca são recriados pelo sistema.

Notificações: renovado (com a nova data), falhou (com o motivo), vencendo, senha inválida e automação pausada.

### 5.5 Contratos IPC (congelados; o `tauri-specta` gera `src/lib/ipc/bindings.ts`)

Comandos, por grupo:
- **Geral**: `app_status`, `library_presets`, `library_validate(url)`.
- **Contas**: `account_list`, `account_create`, `account_update`, `account_delete`, `account_test_login`.
- **Empréstimos**: `loan_list(filter)`, `loan_get(id)`, `loan_set_auto_renew(id, on)`, `loan_mark_returned(id)`.
- **Execução e cota**: `cycle_run_now(account_id, kind: sync|renew_due)`, `quota_status(account_id)`.
- **Agenda**: `schedule_list(filter)`, `schedule_cancel`, `schedule_pause`, `schedule_reactivate`, `schedule_reschedule(id, run_at)`.
- **Histórico**: `history_list(filter, page)`, `history_export(format: csv|json, path)`, `history_clear(before?)`.
- **Estatísticas**: `stats_overview(range)`, `stats_monthly(range)`.
- **Ajustes e dados**: `settings_get`, `settings_set(patch)`, `backup_create(path, passphrase)`, `backup_restore(path, passphrase)`, `data_wipe`.
- **Diagnóstico**: `diagnostics_requests(limit)`.

Eventos: `cycle://progress`, `cycle://finished`, `automation://state`.

### 5.6 Telas (rota → unidade da Onda 2)

- `/onboarding` (W2-04): aviso de privacidade e uso responsável, com consentimento. Escolha da biblioteca (UFTM pré-selecionada; "Avançado" permite endereço customizado, com aviso). Modo automático ou manual. Teste de login (conta na cota). Pedido de permissão de notificações.
- `/` Painel (W2-05): próximos vencimentos, estado da automação, última e próxima renovação, "Sincronizar agora" mostrando a cota e a espera, progresso via `aria-live`.
- `/emprestimos` e `/emprestimos/[id]` (W2-06): lista com filtros; ficha com carimbos (o histórico do livro); liga/desliga a renovação automática; renovar agora; marcar como devolvido.
- `/agenda` (W2-07): CRUD de agendamentos (cancelar, pausar, reagendar dentro da janela permitida, reativar). "Cancelar renovação" significa sempre cancelar uma renovação **agendada**.
- `/historico` (W2-08): tentativas com filtros; exportar em CSV/JSON; apagar.
- `/estatisticas` (W2-09): livros distintos, empréstimos ativos, renovações (total e por mês), taxa de sucesso, dias ganhos, livro mais renovado. Gráficos em SVG acessível com tabela equivalente, seguindo a skill `dataviz`.
- `/contas` (W2-10): CRUD com até 3 contas; trocar senha ou endereço; testar login; excluir com limpeza dos dados.
- `/ajustes` (W2-11):
  - antecedências, horário e notificações;
  - tema (claro, escuro, alto contraste) e movimento;
  - iniciar com o sistema e bloqueio do app;
  - backup e "apagar tudo".
- `/sobre` (W2-12): versão, licenças de terceiros, aviso de privacidade, uso responsável e diagnóstico (log local de requisições, sem credenciais).

### 5.7 Design, acessibilidade e performance (briefing para o `/impeccable teach`)

**Público**: alunos e servidores da UFTM, incluindo pessoas com deficiência visual ou motora. Uso rápido e esporádico; o app precisa transmitir confiança e transparência.

**Direção — neo-brutalismo editorial de "ficha de empréstimo"**:
- papel off-white, tinta preta, bordas de 3 px, sombras duras deslocadas, grid visível;
- **carimbos de data**: cada renovação carimba a ficha;
- empréstimos como lombadas de livro;
- capas geradas a partir do título (sem baixar imagens);
- fontes OFL embutidas: uma grotesca para títulos, uma serifada de livro e uma mono para datas;
- identidade da Editora/UFTM, se fornecida;
- dials do taste-skill: VARIANCE alto, MOTION médio, DENSITY média.

**Movimento**:
- GSAP: timelines, transições, Flip e o carimbo.
- anime.js: micro-interações e SVG.
- Three.js: só em 1 ou 2 momentos decorativos (onboarding e estado vazio):
  - carregado sob demanda e renderizado só quando necessário, com DPR ≤ 1,5;
  - marcado `aria-hidden`, com fallback em SVG estático (WebKitGTK no Linux, sem WebGL ou com movimento reduzido).

**Acessibilidade — WCAG 2.2 AA + eMAG**:
- 100% operável por teclado, com foco visível grosso e alvos de pelo menos 44 px;
- `aria-live` no progresso;
- cor nunca é o único indicador; tamanhos em rem;
- testes com NVDA, TalkBack e VoiceOver.

**Orçamentos de performance**:
- JS inicial ≤ 150 KB gz;
- abertura ≤ 1 s;
- processo na bandeja ≤ 40 MB de RAM;
- instalador desktop ≤ 15 MB;
- Lighthouse de acessibilidade = 100.

## 6. Onda 0 — Fundação (sequencial, sessão normal)

1. Remover `main.js`, `preload.js`, `renderer.js`, `index.html`, `styles.css` e as dependências `electron`, `electron-builder` e `playwright-core`.
2. Criar a base:
   - Tauri 2 + SvelteKit (adapter-static, `ssr=false`) + TypeScript;
   - workspace Cargo com `src-tauri`, `crates/renovador-core` e `plugins/tauri-plugin-renovador-bg` (gerado pelo CLI de plugins, com `android/` e `ios/`; registrado só no mobile);
   - `rust-toolchain.toml`.
3. **Declarar todas as dependências agora**, para que as ondas seguintes não precisem tocar em `Cargo.toml` nem em `package.json`:
   - **Núcleo (Rust)**: `reqwest` (rustls-tls, cookies, gzip), `scraper`, `rusqlite` (`bundled-sqlcipher-vendored-openssl`), `rusqlite_migration`, `chrono`, `chrono-tz`, `tokio`, `serde`/`serde_json`, `thiserror`, `tracing`, `keyring`, `argon2`, `zeroize`, `ed25519-dalek`, `base64`, `url`, `rand`, `async-trait`, `specta`. Para testes: `wiremock`, `tempfile`.
   - **`src-tauri`**: `tauri` 2 com os plugins notification, autostart, single-instance, opener, dialog, fs, log e os; `tauri-specta`.
   - **npm**: `svelte` 5, `@sveltejs/kit`, adapter-static, `vite`, `typescript`, `@tauri-apps/api` e os pacotes JS dos plugins, `gsap`, `animejs`, `three`, `@types/three`, fontes `@fontsource` (as que o DESIGN.md definir), `vitest`, `@testing-library/svelte`, `jsdom`, `@playwright/test`, `@axe-core/playwright`, `svelte-check`.
4. **Contratos do núcleo**, com stubs que retornam `NotImplemented`:
   - `error.rs`;
   - `clock.rs` (`Clock`, `SystemClock`, `FakeClock`);
   - `model.rs` (tipos de domínio, com serde + specta);
   - `guard/mod.rs` (`RequestGate`, `GuardPersistence`, `Permit`, `GuardError`) e `guard/limits.rs` (constantes da 5.2);
   - `store/mod.rs` (API do `Store`) e `store/migrations/0001_init.sql` (schema da 5.3);
   - `keys/mod.rs` (`KeyStore`);
   - `sophia/{mod,model,parser,selectors}.rs` (`Transport`, `SophiaClient`, seletores da 5.1);
   - `planner/mod.rs`;
   - `engine/mod.rs` (`run_cycle` e `CycleReport`);
   - `ffi.rs` (`run_background_cycle(db_path, key) -> json`).
5. **`src-tauri`**:
   - todos os comandos da 5.5 em `commands.rs`, retornando `NotImplemented`;
   - `state.rs`;
   - `tray::init`, `scheduler::start` e `notify::init` chamados no `setup` (stubs);
   - `bindings.ts` gerado e commitado;
   - capabilities mínimas e CSP restrita (`default-src 'self'`);
   - identificador provisório `br.edu.uftm.editora.renovador`, nome "Renovador de Livros".
6. **Frontend**:
   - rotas da 5.6 como placeholders;
   - `+layout.svelte` usando `lib/components/AppShell.svelte` (stub);
   - `lib/ipc/index.ts`, com um helper de mock para testes;
   - `lib/motion/index.ts` (API stub: `reveal`, `stamp`, `pageTransition`, `prefersReducedMotion`);
   - `lib/i18n/pt-BR/<tela>.ts`, **um arquivo por tela**;
   - `styles/tokens.css` e `styles/base.css`;
   - configuração do vitest e do Playwright;
   - scripts `check`, `test`, `test:e2e`, `build` e `tauri`.
7. **Licença**:
   - trocar `LICENSE` por `LICENSE-MIT` + `LICENSE-APACHE` e declarar `"MIT OR Apache-2.0"` no `package.json` e no `Cargo.toml`;
   - no `.gitignore`, passar a versionar `package-lock.json` e `Cargo.lock`.
8. **Skills e contexto de design** (interativo, com o usuário):
   - instalar no projeto `npx skills add pbakaus/impeccable` e `npx skills add https://github.com/Leonxlnx/taste-skill --skill design-taste-frontend`, e commitar com as licenças delas;
   - rodar `/impeccable teach` com o briefing da 5.7 para gerar `DESIGN.md` (paletas clara, escura e de alto contraste, tipografia, grid, bordas e sombras, movimento, componentes).
9. Gerar os ícones com `npx tauri icon icon.png`.

**Aceite**:
- `npm run tauri dev` abre o app e navega por todas as rotas.
- `cargo test --workspace`, `npm run check`, `npm run test` e `npm run build` passam.
- `npm run tauri build` gera o instalador NSIS.
- `npm run tauri android build --debug` compila (depois de P1).
- `DESIGN.md` e `bindings.ts` estão commitados.

### Notas da Onda 0 (leia antes das Ondas 1 e 2)

Desvios e convenções descobertos ao montar a fundação:

- **SvelteKit 3**: `$lib` foi removido. Use `#lib/...` (campo `imports` do `package.json`) **com extensão explícita**: `#lib/routes.ts`, `#lib/motion/index.ts`, `#lib/components/Button.svelte`.
- **`Id` é `i32`** (não `i64`): o `specta-typescript` recusa `i64`. O `Store` converte os rowids do SQLite.
- **Sem `rusqlite_migration`** (incompatível com rusqlite 0.42): o `Store` aplica `store::MIGRATIONS` via `PRAGMA user_version`.
- **Bindings**: `cargo run -p renovador --bin export-bindings` regenera `src/lib/ipc/bindings.ts` (também roda no `tauri dev`). Testes de lib do `src-tauri` não rodam no Windows (STATUS_ENTRYPOINT_NOT_FOUND); teste os comandos via `renovador-core` ou binários.
- **Parâmetros dos comandos** não podem ter prefixo `_`: o nome vira a chave do IPC.
- **Eventos** no frontend: `cycleProgressEvent`, `cycleFinishedEvent`, `automationStateEvent` (equivalem a `cycle://progress`, `cycle://finished`, `automation://state`).
- **Windows**: rode `vitest` num terminal cujo caminho comece com `C:\` maiúsculo; com `c:\` o Vitest carrega duas cópias e todo teste falha. O build do SQLCipher precisa do Strawberry Perl antes do perl do Git no `PATH` (`C:\Strawberry\perl\bin`).
- **Plugin mobile**: só o crate Rust mínimo existe; os projetos `android/` e `ios/` são criados na W1-11 (`tauri plugin new` exige terminal interativo).

## 7. Onda 1 — `/batch` (11 unidades independentes)

Todos os caminhos de `crates/...` abaixo são relativos a `crates/renovador-core/`.

**W1-01 · guard**
- Arquivos: `src/guard/**`, `tests/guard_*.rs`.
- Fazer: implementar `RequestGate` sobre `GuardPersistence` (com implementação em memória) e `Clock`:
  - semáforo global, espaçamento + jitter e cotas por conta, instalação e dia;
  - espera mínima entre sessões manuais;
  - breaker e `Retry-After`;
  - validação de host;
  - política remota (verificação Ed25519 e merge "só aperta").
- Aceite: testes com `FakeClock` cobrindo cada linha da tabela 5.2, inclusive "reiniciar não zera a cota" e "a política remota não afrouxa".

**W1-02 · store**
- Arquivos: `src/store/**` (exceto `0001_init.sql`), `tests/store_*.rs`.
- Fazer:
  - abrir o SQLCipher com a chave (WAL, `foreign_keys`) e rodar as migrações;
  - repositórios CRUD de todas as tabelas;
  - implementação de `GuardPersistence`;
  - backup e restauração (`sqlcipher_export` protegido por senha) e "apagar tudo".
- Aceite: testes de ida e volta dos dados; abrir sem a chave falha; o arquivo não tem o cabeçalho SQLite.

**W1-03 · keys (desktop)**
- Arquivos: `src/keys/**`, `tests/keys_*.rs`.
- Fazer: implementar `KeyStore` com `keyring` (Windows, macOS, Linux) e o fallback de senha mestra Argon2id; geração da chave e `zeroize`.
- Aceite: testes com o keyring mockado.

**W1-04 · planner**
- Arquivos: `src/planner/**`, `tests/planner_*.rs`.
- Fazer: as regras da 5.4 como funções puras:
  - data de renovação, lembretes e sincronização de descoberta;
  - janela 07h–21h com semente por instalação;
  - catch-up e livros vencidos;
  - respeito aos agendamentos `origin=user`.

  Fuso America/Sao_Paulo.
- Aceite: testes de tabela cobrindo vencimento hoje, amanhã e passado, fim de mês, agendamentos pausados e cancelados.

**W1-05 · sophia client**
- Arquivos: `src/sophia/client.rs`, `src/sophia/detect.rs`, `tests/client_*.rs`.
- Fazer:
  - transporte com reqwest (cookies, gzip, timeout de 15 s, User-Agent honesto);
  - fluxo da 5.1 (login, lista, renovação, logout), passando **cada requisição** pelo `RequestGate` (fake nos testes);
  - detecção de CAPTCHA, desafio do Cloudflare e host que não é Sophia;
  - no máximo 1 nova tentativa em GET idempotente quando houver erro de rede.
- Aceite: testes com wiremock (login ok e com erro, lista, renovação, CAPTCHA, Cloudflare, 429, 503), sem nenhuma chamada à rede real.

**W1-06 · sophia parser** (requer P3; se as fixtures não existirem, pular esta unidade)
- Arquivos: `src/sophia/parser.rs`, `src/sophia/selectors.rs`, `tests/parser_*.rs`.
- Fazer:
  - extrair da lista: título, link, data prevista (dd/mm/aaaa), nº de renovações e `item_key`;
  - classificar a resposta da renovação (sucesso com nova data, reserva, limite, bloqueio, atraso);
  - ler o JSON do login.

  Estrutura desconhecida retorna `ParseError`, sem chutar.
- Aceite: testes sobre todas as fixtures.

**W1-07 · design system**
- Arquivos: `src/styles/**`, `src/lib/components/**`, `src/lib/motion/**`, `src/routes/dev/kit/+page.svelte` (só em desenvolvimento), `tests/ui/components/**`.
- Fazer, seguindo o DESIGN.md:
  - tokens dos temas claro, escuro e alto contraste, e fontes embutidas;
  - componentes: AppShell, Button, IconButton, Ficha (card), Stamp, Spine, campos de texto e senha, Toggle, Select, Dialog, Toast (região `aria-live`), Tabs, DataTable, EmptyState, Skeleton, StatusPill e ProgressSteps;
  - `lib/motion` com GSAP e anime.js, respeitando o movimento reduzido.
- Aceite: axe zerado em cada componente; teste de teclado; a página `/dev/kit` renderiza nos três temas.

**W1-08 · Three hero**
- Arquivos: `src/lib/three/**`, `tests/ui/three/**`.
- Fazer: componente `<HeroBook />` com cena decorativa:
  - carregada sob demanda e renderizada só quando necessário, com DPR ≤ 1,5;
  - pausa quando a janela fica oculta;
  - detecta WebGL e cai para SVG estático;
  - `aria-hidden` e respeito ao movimento reduzido.
- Aceite: `three` fora do chunk inicial (conferir no relatório do build); o fallback funciona sem WebGL.

**W1-09 · documentação, aspectos legais e README**
- Arquivos: `docs/**` (exceto `PLANO-V2.md`), `README.md`.
- Fazer:
  - `docs/legal/analise-preliminar.md` (seção 11 + fontes) e `docs/legal/modelo-oficio-anuencia.md`;
  - `docs/legal/aviso-de-privacidade.md` (LGPD) e `docs/legal/uso-responsavel.md`;
  - `docs/arquitetura.md` e `docs/guia-de-manutencao.md` (atualizar seletores e fixtures; gerar e rotacionar o par Ed25519 e publicar a política remota);
  - esqueleto de `docs/manual-do-usuario.md`;
  - reescrever o `README.md`: corrigir a afirmação sobre credenciais, nova licença e aviso de que o projeto não tem vínculo com a Sophia/Prima ("Sophia" é marca de terceiros).
- Aceite: links válidos; nenhuma afirmação contrária à especificação.

**W1-10 · CI/CD**
- Arquivos: `.github/**`.
- Fazer:
  - `ci.yml`: fmt, clippy, test, check, vitest e build, numa matriz Windows/Ubuntu/macOS;
  - job de build Android debug;
  - `cargo-deny` com allowlist de licenças (incluindo a exceção do GSAP) e `npm audit`;
  - Dependabot para npm e cargo;
  - `release.yml`: `tauri-action` gerando **release em rascunho**, só por disparo manual e protegido pelo environment `publicacao`, com aprovação obrigatória (é o gate jurídico).
- Aceite: o workflow passa nos PRs.

**W1-11 · plugin mobile**
- Arquivos: `plugins/tauri-plugin-renovador-bg/**`.
- Fazer:
  - **Kotlin**: chave AES-GCM no AndroidKeyStore, envolvendo a chave do banco; `RenewWorker` (CoroutineWorker) e a API de agendamento (periódico de 6 h + execução única no horário planejado); ponte JNI para `ffi::run_background_cycle`.
  - **Swift**: esqueleto de Keychain e de registro no `BGTaskScheduler`.
  - Comandos do plugin: `schedule_next(at)` e `get_db_key(`C:\Strawberry\perl\bin`)`.
- Aceite: build Android de debug compila; testes JVM da lógica de agendamento.

## 8. Onda 2 — `/batch` (12 unidades)

**W2-01 · engine + FFI**
- Arquivos: `src/engine/**`, `src/ffi.rs`, `tests/engine_*.rs` (em `crates/renovador-core/`).
- Fazer: `run_cycle(account, trigger)`, nesta ordem:
  1. consultar a política remota e o gate;
  2. login e lista;
  3. sincronizar o banco (upsert, detectar devoluções);
  4. o planner decide o que renovar;
  5. renovar cada livro e confirmar com novo GET da lista;
  6. gravar `renewal_event` e os novos agendamentos;
  7. logout e geração do `CycleReport`.

  Também implementar `run_background_cycle`.
- Aceite: testes com wiremock em todos os cenários da seção 10, verificando o estado do banco, o nº de requisições, o espaçamento (`FakeClock`) e o breaker.

**W2-02 · comandos IPC**
- Arquivos: `src-tauri/src/commands.rs` (só os corpos das funções), `src-tauri/src/state.rs`.
- Fazer: implementar todos os comandos da 5.5:
  - reagendar só dentro da janela permitida;
  - exportar via diálogo de arquivo;
  - emitir o evento `cycle://progress`.
- Aceite: testes de integração dos comandos com banco temporário e engine apontando para o mock.

**W2-03 · segundo plano no desktop**
- Arquivos: `src-tauri/src/{tray,scheduler,notify}.rs`.
- Fazer:
  - bandeja com Abrir / Sincronizar agora / Pausar automação / Sair;
  - fechar a janela libera o WebView;
  - autostart e foco na instância única;
  - loop de 30 min com catch-up;
  - notificações da 5.4.
- Aceite: teste manual descrito no PR (Windows); testes unitários do loop com relógio falso.

**W2-04 a W2-12 · telas da 5.6**, uma unidade por tela (onboarding, painel, empréstimos, agenda, histórico, estatísticas, contas, ajustes, sobre).
- Arquivos de cada unidade: `src/routes/<tela>/**`, `src/lib/features/<tela>/**`, `src/lib/i18n/pt-BR/<tela>.ts`, `tests/ui/<tela>/**`.
- Fazer: usar o kit de componentes, `lib/motion` e o IPC tipado. Nos testes, mockar o IPC com `@tauri-apps/api/mocks`. Estatísticas seguem a skill `dataviz`. Onboarding e estado vazio usam o `<HeroBook />`.
- Aceite: axe zerado; fluxo completo só por teclado no Playwright; textos em pt-BR.

## 9. Onda 3 — Integração, empacotamento, QA e entrega (sessão normal)

- **Android**:
  - ligar o plugin ao engine via FFI;
  - fluxo de permissão `POST_NOTIFICATIONS`;
  - testar o WorkManager num aparelho real (Doze e modo avião);
  - assinatura com a keystore da Editora, fora do repositório;
  - gerar APK e AAB.
- **iOS**, se houver Mac e conta Apple: BGTask, Keychain e notificações; TestFlight.
- **Empacotamento**:
  - NSIS/MSI no Windows;
  - AppImage, deb e rpm no Linux;
  - dmg no macOS, via CI;
  - `THIRD-PARTY-NOTICES.md` gerado com `cargo about` + licenças npm, incluindo a do GSAP.
- **QA**:
  - `/impeccable audit`, depois `critique`, depois `polish`;
  - axe em todas as telas;
  - testes com NVDA, TalkBack e VoiceOver;
  - orçamentos da 5.7;
  - `/security-review`.
- **Entrega**:
  - manual do usuário com capturas de tela;
  - par de chaves Ed25519 da política remota gerado e guardado pela Editora;
  - checklist de release;
  - **publicar só depois da anuência por escrito.**

### Pendências herdadas da Onda 2

Lacunas encontradas na revisão da Onda 2. As que mudavam contratos congelados foram feitas na Onda 3 (branch `onda3/pendencias`), com o motivo explicado no PR.

Resolvidas:
- **Horário nos Ajustes** (#19): `Settings.renew_period` (qualquer hora, manhã, tarde ou noite). O minuto continua sorteado dentro do período, e mudar o período replaneja a agenda sem acessar a rede.
- **Antecedências** (#19): ficam por conta, em `/contas` (`renew_days_before` 0–3, `remind_days_before` 0–7, padrão 1 e 2). O planner passou a usar o lembrete da conta, que antes ignorava, preso em D−2.
- **Renovar um empréstimo só** (#22): comando `loan_renew(loan_id)` (`Engine::renew_loan`), com o mesmo gate e a mesma cota de um ciclo manual.
- **Estatísticas no fuso local** (#25): intervalo e meses no fuso de Brasília; `success_rate` é `Option<f64>`.
- **Pausa da bandeja** (#26): `Settings.automation_paused`, lida ao iniciar e mostrada no `app_status`.

Ainda abertas:
- **Parser real (W1-06)**: os testes do engine usam parsers falsos; ligar os reais assim que existirem os fixtures da P3.
- **Teste manual do segundo plano no Windows** (#26): seguir o roteiro do PR (bandeja, autostart, instância única, catch-up e notificações).
- **Teste do Android** (#29): rodar o APK do CI num aparelho ou emulador. Neste PC a virtualização está desligada no firmware, e o emulador x86_64 não sobe.
- **Documentar no manual**: quando a renovação volta `Reserved`, `LimitReached`, `Blocked` ou `Overdue`, o engine desliga o `auto_renew` daquele empréstimo, e o usuário religa pela ficha.

## 10. Verificação end-to-end

- **Automatizada**:
  - `cargo test --workspace`: parser (fixtures), limites (`FakeClock`), planner, migrações, breaker, host e engine.
  - Cenários do servidor mock: sucesso, senha errada, reserva, limite atingido, 429 com `Retry-After`, 503, CAPTCHA, desafio do Cloudflare, HTML alterado e host que não é Sophia. Em cada um, conferir o comportamento, o nº de requisições e o espaçamento.
  - `.db` aberto com `sqlite3` sem a chave → "file is not a database".
  - `svelte-check`, vitest, Playwright + axe (IPC mockado) e Lighthouse.
  - `three` fora do chunk inicial.
- **Manual (usuário, conta real)**:
  - no Windows e num Android físico, renovar 1 livro de verdade e conferir no site e no e-mail de recibo do Sophia;
  - reiniciar no meio da sessão: a cota deve persistir;
  - cortar a rede: o breaker deve abrir;
  - Android em Doze e em modo avião: o WorkManager deve voltar a executar depois.

## 11. Gate jurídico e governança (não bloqueia código; bloqueia publicação)

**Conclusão da pesquisa** (não é parecer jurídico): **nenhum impedimento legal encontrado.**
- O Sophia oferece renovação por **autoatendimento** no Terminal Web (Descritivo Funcional, 20.25). A biblioteca configura os limites e o servidor os aplica (13.12.3.1 e 5.1.15).
- A UFTM permite renovar pela internet (5 livros, 7 dias, até 15 renovações sem reserva — *confirmar*).
- O `robots.txt` libera `/5362/mobile/` para agentes genéricos (bloqueia só `/gerenciador/`, `/terminal/`, `/terminalri/` e crawlers de IA).
- Não há termos públicos que proíbam automação.
- A API oficial não cobre renovação.
- Leis:
  - CP art. 154-A exige acesso sem autorização; credencial própria, sem burlar segurança, não se enquadra.
  - CP art. 266 §1º fundamenta os limites anti-DDoS.
  - LGPD art. 4º, I: o tratamento é local, feito pelo próprio titular.

**Obrigações**:
- anuência por escrito da Biblioteca/SIBi UFTM antes de publicar (também atende à diretriz 5.2.2 da App Store);
- nunca contornar CAPTCHA ou bloqueio (o Sophia permite ativar CAPTCHA — itens 20.2.3.1 e 20.37);
- confirmar a titularidade e o relicenciamento com a Editora/NIT (hoje o autor é só Hebert Almeida; as versões antigas continuam GPL-3.0);
- outras bibliotecas Sophia são experimentais e ficam sob responsabilidade do usuário.

**Critério de parada**: se a Biblioteca UFTM ou a Sophia negarem ou pedirem para cessar, o projeto para e o kill switch desliga a automação em todas as instalações.

**Fontes**:
- robots.txt: https://biblioteca.sophia.com.br/robots.txt
- Descritivo Funcional: https://www.ifb.edu.br/attachments/article/39100/Descritivo%20Funcional%20do%20Sophia%20Biblioteca%20Web.pdf
- API Web de Integração: https://ifb.edu.br/attachments/article/39100/API%20Web%20de%20Integra%C3%A7%C3%A3o.pdf
- Regras da UFTM: https://sistemas.uftm.edu.br/integrado/sistemas/pub/publicacao.html?secao=1527&publicacao=10973
- Política de privacidade da Sophia: https://sophia.com.br/politica-de-privacidade/
- Licença do GSAP: https://gsap.com/community/standard-license/
- WorkManager no Tauri (issue #14530): https://github.com/tauri-apps/tauri/issues/14530

## 12. Riscos

- O HTML pós-login ainda não foi visto; depende da P3.
- Outras bibliotecas Sophia podem ter outra versão do HTML.
- O WebGL no WebKitGTK (Linux) pode falhar → fallback estático.
- O iOS é melhor esforço e exige Mac + conta Apple institucional (D-U-N-S).
- O SQLCipher com OpenSSL vendorizado exige Strawberry Perl no Windows e NDK no Android.
- Conflitos de merge no `/batch` → mitigados pela posse exclusiva de arquivos e pelos contratos congelados na Onda 0.
