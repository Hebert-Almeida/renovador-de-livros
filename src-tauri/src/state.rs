//! Estado compartilhado do app (W2-02).
//!
//! Este módulo **não depende do Tauri**: a lógica dos comandos IPC mora aqui e
//! `commands.rs` só delega. Assim os testes de integração (`tests/ipc_commands.rs`)
//! incluem este arquivo direto, sem linkar o Tauri (que não roda em testes no Windows).

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Duration, Utc};
use renovador_core::clock::{Clock, SystemClock};
use renovador_core::engine::{CycleOutcome, Engine};
use renovador_core::guard::{
    DefaultGate, RemotePolicy, RequestGate, check_account_count, validate_library_url,
};
use renovador_core::keys::{self, KeyStore, OsKeyStore};
use renovador_core::model::*;
use renovador_core::sophia::client::{ClientConfig, HttpSophiaClient, ReqwestTransport};
use renovador_core::sophia::{LoginResult, SophiaClient, Transport};
use renovador_core::store::{DbKey, Store};
use renovador_core::{Error, Result, planner};

/// Nome do arquivo do banco dentro do diretório de dados.
pub const DB_FILE: &str = "renovador.db";
/// Sal do fallback Argon2id (`keys/argon2.salt`, não secreto).
pub const SALT_FILE: &str = "argon2.salt";
/// Política remota em cache (`policy/policy.json` + `policy/policy.json.sig`).
pub const POLICY_DIR: &str = "policy";
/// Reagendamentos sem empréstimo (sync) só até este horizonte.
pub const MAX_RESCHEDULE_DAYS: i64 = 30;
/// Tamanho do lote ao exportar o histórico.
const EXPORT_BATCH: u32 = 500;

/// Recebe o progresso de um ciclo (o Tauri emite `cycle://progress`).
pub type ProgressSink = Arc<dyn Fn(CycleProgress) + Send + Sync>;

/// Dependências injetáveis: produção usa o cofre do SO e o reqwest; testes usam
/// cofre em memória e transporte falso (nunca o site real).
#[derive(Clone)]
pub struct AppConfig {
    pub data_dir: PathBuf,
    pub keystore: Arc<dyn KeyStore>,
    pub clock: Arc<dyn Clock>,
    pub transport: Arc<dyn Transport>,
    pub client_config: ClientConfig,
    /// Chave pública Ed25519 da política remota (`None` = sem política remota).
    pub policy_public_key: Option<[u8; 32]>,
}

impl AppConfig {
    /// Configuração de produção.
    #[allow(dead_code)] // usada só pelo app (não pelos testes)
    pub fn production(data_dir: PathBuf) -> Result<Self> {
        Ok(Self {
            data_dir,
            keystore: Arc::new(OsKeyStore),
            clock: Arc::new(SystemClock),
            transport: Arc::new(ReqwestTransport::new()?),
            client_config: ClientConfig::default(),
            policy_public_key: None,
        })
    }

    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join(DB_FILE)
    }
}

/// `KeyStore` do fallback de senha mestra: a chave é derivada (Argon2id) e
/// nunca guardada; só o sal é persistido em `keys/`.
pub struct PassphraseKeyStore {
    key: DbKey,
}

impl PassphraseKeyStore {
    /// Deriva a chave com o sal de `data_dir/keys/` (criado na primeira vez).
    #[allow(dead_code)] // fallback do Linux sem Secret Service (ligado na Onda 3)
    pub fn new(data_dir: &Path, passphrase: &str) -> Result<Self> {
        let salt = load_or_create_salt(data_dir)?;
        Ok(Self {
            key: keys::derive_from_passphrase(passphrase, &salt)?,
        })
    }
}

impl KeyStore for PassphraseKeyStore {
    fn load(&self) -> Result<Option<DbKey>> {
        Ok(Some(self.key.clone()))
    }
    fn store(&self, _key: &DbKey) -> Result<()> {
        Ok(())
    }
    fn delete(&self) -> Result<()> {
        Ok(())
    }
}

/// Lê o sal do Argon2id ou cria um novo em `data_dir/keys/argon2.salt`.
pub fn load_or_create_salt(data_dir: &Path) -> Result<[u8; 16]> {
    let dir = data_dir.join("keys");
    let path = dir.join(SALT_FILE);
    match std::fs::read(&path) {
        Ok(bytes) => bytes
            .try_into()
            .map_err(|_| Error::Keys("sal do Argon2 corrompido".into())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir_all(&dir).map_err(io_err(&dir))?;
            let salt = keys::generate_salt();
            write_atomic(&path, &salt)?;
            Ok(salt)
        }
        Err(e) => Err(io_err(&path)(e)),
    }
}

/// Lê a política remota em cache e só a aceita com assinatura válida.
/// Formato (guia de manutenção): `policy.json` + `policy.json.sig` (Ed25519).
pub fn load_cached_policy(data_dir: &Path, public_key: Option<&[u8; 32]>) -> RemotePolicy {
    let Some(pk) = public_key else {
        return RemotePolicy::default();
    };
    let dir = data_dir.join(POLICY_DIR);
    let (Ok(json), Ok(sig)) = (
        std::fs::read(dir.join("policy.json")),
        std::fs::read(dir.join("policy.json.sig")),
    ) else {
        return RemotePolicy::default();
    };
    RemotePolicy::verify_and_parse(&json, &sig, pk).unwrap_or_else(|e| {
        tracing::warn!("política remota em cache rejeitada: {e}");
        RemotePolicy::default()
    })
}

/// Serviços montados sobre um banco aberto.
#[derive(Clone)]
struct Services {
    store: Arc<Store>,
    gate: Arc<DefaultGate>,
    client: Arc<dyn SophiaClient>,
    engine: Arc<Engine>,
    policy: RemotePolicy,
}

impl Services {
    fn build(config: &AppConfig, store: Store) -> Self {
        let policy = load_cached_policy(&config.data_dir, config.policy_public_key.as_ref());
        Self::build_from(config, Arc::new(store), policy)
    }

    fn build_from(config: &AppConfig, store: Arc<Store>, policy: RemotePolicy) -> Self {
        let gate =
            Arc::new(DefaultGate::new(store.clone(), config.clock.clone()).with_policy(&policy));
        let client: Arc<dyn SophiaClient> = Arc::new(HttpSophiaClient::new(
            config.transport.clone(),
            gate.clone(),
            config.client_config.clone(),
        ));
        let engine = Arc::new(Engine::new(
            gate.clone(),
            client.clone(),
            config.clock.clone(),
        ));
        Self {
            store,
            gate,
            client,
            engine,
            policy,
        }
    }
}

struct Inner {
    config: AppConfig,
    services: Services,
}

#[derive(Default)]
pub struct AppState {
    inner: RwLock<Option<Inner>>,
    progress: RwLock<Option<ProgressSink>>,
}

fn not_ready() -> Error {
    Error::Store("o banco ainda não foi aberto".into())
}

fn io_err(path: &Path) -> impl Fn(std::io::Error) -> Error + '_ {
    move |e| Error::Store(format!("{}: {e}", path.display()))
}

/// Grava num `.tmp` e renomeia (não deixa arquivo pela metade).
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    let res = std::fs::File::create(&tmp)
        .and_then(|mut f| {
            f.write_all(bytes)?;
            f.sync_all()
        })
        .and_then(|()| std::fs::rename(&tmp, path));
    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res.map_err(io_err(path))
}

impl AppState {
    /// Abre o banco (criando a chave na primeira execução) e monta os serviços.
    pub fn init(&self, config: AppConfig) -> Result<()> {
        std::fs::create_dir_all(&config.data_dir).map_err(io_err(&config.data_dir))?;
        let key = keys::load_or_create(config.keystore.as_ref())?;
        let store = Store::open(&config.db_path(), &key)?;
        let services = Services::build(&config, store);
        *self.inner.write().unwrap_or_else(|p| p.into_inner()) = Some(Inner { config, services });
        Ok(())
    }

    pub fn set_progress_sink(&self, sink: ProgressSink) {
        *self.progress.write().unwrap_or_else(|p| p.into_inner()) = Some(sink);
    }

    fn services(&self) -> Result<Services> {
        self.inner
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .as_ref()
            .map(|i| i.services.clone())
            .ok_or_else(not_ready)
    }

    fn store(&self) -> Result<Arc<Store>> {
        Ok(self.services()?.store)
    }

    /// Acesso ao banco para o segundo plano (bandeja, scheduler, notificações);
    /// `None` enquanto o banco não foi aberto.
    pub fn with_store<T>(&self, f: impl FnOnce(&Store) -> T) -> Option<T> {
        self.store().ok().map(|s| f(&s))
    }

    fn now(&self) -> Result<DateTime<Utc>> {
        self.inner
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .as_ref()
            .map(|i| i.config.clock.now())
            .ok_or_else(not_ready)
    }

    // --- Geral ---

    pub fn app_status(&self) -> Result<AppStatus> {
        let s = self.services()?;
        let accounts = s.store.account_list()?;
        let settings = s.store.settings_get()?;
        let next_run_at = s
            .store
            .schedule_list(&ScheduleFilter {
                status: Some(ScheduleStatus::Pending),
                ..Default::default()
            })?
            .into_iter()
            .map(|sc| sc.run_at)
            .min();
        Ok(AppStatus {
            version: renovador_core::VERSION.to_owned(),
            accounts: accounts.len() as u32,
            automation: automation_state(&accounts, &s.policy),
            next_run_at,
            onboarding_done: settings.onboarding_done,
        })
    }

    pub async fn library_validate(&self, url: &str) -> Result<LibraryValidation> {
        let fail = |reason: String| LibraryValidation {
            ok: false,
            normalized_url: None,
            reason: Some(reason),
        };
        let normalized = match normalize_library_url(url) {
            Ok(u) => u,
            Err(e) => return Ok(fail(e.to_string())),
        };
        let client = self.services()?.client;
        match client.validate_library(&normalized).await {
            Ok(_) => Ok(LibraryValidation {
                ok: true,
                normalized_url: Some(normalized),
                reason: None,
            }),
            // Cota/espera não dizem nada sobre o endereço: sobe como erro.
            Err(e @ Error::Guard(_)) => Err(e),
            Err(e) => Ok(fail(e.to_string())),
        }
    }

    // --- Contas ---

    pub fn account_list(&self) -> Result<Vec<Account>> {
        self.store()?.account_list()
    }

    pub fn account_create(&self, mut account: NewAccount) -> Result<Account> {
        let store = self.store()?;
        check_account_count(store.account_list()?.len() as u32)?;
        account.library_base_url = normalize_library_url(&account.library_base_url)?;
        if account.login.trim().is_empty() {
            return Err(Error::Invalid("informe o login".into()));
        }
        store.account_create(&account)
    }

    pub fn account_update(&self, id: Id, mut patch: AccountPatch) -> Result<Account> {
        let s = self.services()?;
        if let Some(url) = &patch.library_base_url {
            patch.library_base_url = Some(normalize_library_url(url)?);
        }
        let credentials_changed = patch.password.is_some() || patch.library_base_url.is_some();
        let acc = s.store.account_update(id, &patch)?;
        if credentials_changed {
            // Senha ou endereço novos: o breaker da conta recomeça do zero.
            s.gate.resume_account(id)?;
        }
        Ok(acc)
    }

    pub fn account_delete(&self, id: Id) -> Result<()> {
        self.store()?.account_delete(id)
    }

    /// Faz login e logout (conta na cota). `Ok(false)` = login ou senha inválidos.
    pub async fn account_test_login(&self, id: Id) -> Result<bool> {
        let s = self.services()?;
        let acc = s.store.account_get(id)?;
        let password = s
            .store
            .account_password(id)?
            .ok_or_else(|| Error::Invalid("a conta não tem senha salva".into()))?;
        // O gate valida o endereço completo e guarda o host normalizado.
        let ticket = s
            .gate
            .begin_session(id, &acc.library_base_url, Trigger::Manual)
            .await?;
        let result = s
            .client
            .login(&ticket, &acc.library_base_url, &acc.login, &password)
            .await;
        let out = match result {
            Ok(LoginResult::Ok(session)) => {
                if let Err(e) = s.client.logout(&ticket, &session).await {
                    tracing::warn!("logout falhou após o teste de login: {e}");
                }
                Ok(true)
            }
            Ok(LoginResult::InvalidCredentials { .. }) | Err(Error::InvalidCredentials) => {
                Ok(false)
            }
            Err(e) => Err(e),
        };
        s.gate.end_session(ticket);
        if let Ok(ok) = out {
            let status = if ok {
                (acc.status == AccountStatus::InvalidPassword).then_some(AccountStatus::Active)
            } else {
                Some(AccountStatus::InvalidPassword)
            };
            if let Some(status) = status {
                s.store.account_update(
                    id,
                    &AccountPatch {
                        status: Some(status),
                        ..Default::default()
                    },
                )?;
            }
        }
        out
    }

    // --- Empréstimos ---

    pub fn loan_list(&self, filter: &LoanFilter) -> Result<Vec<LoanView>> {
        self.store()?.loan_list(filter)
    }

    pub fn loan_get(&self, id: Id) -> Result<LoanDetail> {
        self.store()?.loan_get(id)
    }

    pub fn loan_set_auto_renew(&self, id: Id, on: bool) -> Result<()> {
        self.store()?.loan_set_auto_renew(id, on)
    }

    pub fn loan_mark_returned(&self, id: Id) -> Result<()> {
        self.store()?.loan_mark_returned(id)
    }

    // --- Execução e cota ---

    /// Roda um ciclo manual. O progresso vai para o `ProgressSink`.
    /// "Sincronizar agora" da interface: sempre conta como manual.
    pub async fn cycle_run_now(&self, account_id: Id, kind: CycleKind) -> Result<CycleReport> {
        self.run_cycle(account_id, kind, Trigger::Manual)
            .await
            .map(|o| o.report)
    }

    /// Ciclo com o gatilho real (o scheduler passa `Auto`/`CatchUp`, que não
    /// gastam a cota manual) e o resultado completo, com as notificações.
    pub async fn run_cycle(
        &self,
        account_id: Id,
        kind: CycleKind,
        trigger: Trigger,
    ) -> Result<CycleOutcome> {
        let s = self.services()?;
        s.store.account_get(account_id)?;
        let sink = self
            .progress
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        let progress = move |p: CycleProgress| {
            if let Some(sink) = &sink {
                sink(p);
            }
        };
        s.engine
            .run_cycle_outcome(&s.store, account_id, kind, trigger, None, &progress)
            .await
    }

    pub fn quota_status(&self, account_id: Id) -> Result<QuotaStatus> {
        let s = self.services()?;
        s.store.account_get(account_id)?;
        s.gate.quota(account_id)
    }

    // --- Agenda ---

    pub fn schedule_list(&self, filter: &ScheduleFilter) -> Result<Vec<Schedule>> {
        self.store()?.schedule_list(filter)
    }

    fn schedule_get(store: &Store, id: Id) -> Result<Schedule> {
        store
            .schedule_list(&ScheduleFilter::default())?
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| Error::NotFound(format!("agendamento {id}")))
    }

    fn loan_due(store: &Store, s: &Schedule) -> Result<Option<chrono::NaiveDate>> {
        s.loan_id
            .map(|l| store.loan_get(l).map(|d| d.view.loan.due_date))
            .transpose()
    }

    fn schedule_transition(
        store: &Store,
        current: &Schedule,
        allowed_from: &[ScheduleStatus],
        to: ScheduleStatus,
    ) -> Result<Schedule> {
        if !allowed_from.contains(&current.status) {
            return Err(Error::Invalid(format!(
                "agendamento {} não pode passar de {} para {}",
                current.id,
                current.status.as_str(),
                to.as_str()
            )));
        }
        store.schedule_set_status(current.id, to, ScheduleOrigin::User)
    }

    /// Cancela uma renovação **agendada** (ou outro agendamento pendente/pausado).
    pub fn schedule_cancel(&self, id: Id) -> Result<Schedule> {
        let store = self.store()?;
        let current = Self::schedule_get(&store, id)?;
        Self::schedule_transition(
            &store,
            &current,
            &[ScheduleStatus::Pending, ScheduleStatus::Paused],
            ScheduleStatus::Cancelled,
        )
    }

    pub fn schedule_pause(&self, id: Id) -> Result<Schedule> {
        let store = self.store()?;
        let current = Self::schedule_get(&store, id)?;
        Self::schedule_transition(
            &store,
            &current,
            &[ScheduleStatus::Pending],
            ScheduleStatus::Paused,
        )
    }

    /// Reativa um agendamento pausado ou cancelado. Se o horário já passou, ele
    /// vai para o próximo instante dentro da janela (catch-up), desde que isso
    /// ainda respeite a janela permitida.
    pub fn schedule_reactivate(&self, id: Id) -> Result<Schedule> {
        let now = self.now()?;
        let store = self.store()?;
        let current = Self::schedule_get(&store, id)?;
        let new_run_at = if current.run_at <= now {
            let at = planner::next_in_window(now + Duration::minutes(1));
            let as_pending = Schedule {
                status: ScheduleStatus::Pending,
                ..current.clone()
            };
            validate_reschedule(&as_pending, Self::loan_due(&store, &current)?, at, now)?;
            Some(at)
        } else {
            None
        };
        let s = Self::schedule_transition(
            &store,
            &current,
            &[ScheduleStatus::Paused, ScheduleStatus::Cancelled],
            ScheduleStatus::Pending,
        )?;
        match new_run_at {
            Some(at) => store.schedule_reschedule(id, at),
            None => Ok(s),
        }
    }

    /// Reagenda só dentro da janela permitida (ver [`validate_reschedule`]).
    pub fn schedule_reschedule(&self, id: Id, run_at: DateTime<Utc>) -> Result<Schedule> {
        let now = self.now()?;
        let store = self.store()?;
        let current = Self::schedule_get(&store, id)?;
        validate_reschedule(&current, Self::loan_due(&store, &current)?, run_at, now)?;
        store.schedule_reschedule(id, run_at)
    }

    // --- Histórico ---

    pub fn history_list(&self, filter: &HistoryFilter, page: Page) -> Result<HistoryPage> {
        self.store()?.history_list(filter, page)
    }

    /// Exporta todo o histórico para `path` (escolhido no diálogo de arquivo do
    /// frontend). Devolve o número de tentativas exportadas.
    pub fn history_export(&self, format: ExportFormat, path: &str) -> Result<u32> {
        let path = Path::new(path);
        if path.as_os_str().is_empty() || !path.is_absolute() {
            return Err(Error::Invalid("caminho de exportação inválido".into()));
        }
        let store = self.store()?;
        let db = store.path();
        let same = path == db
            || matches!((path.canonicalize(), db.canonicalize()), (Ok(a), Ok(b)) if a == b);
        if same {
            return Err(Error::Invalid(
                "a exportação não pode sobrescrever o banco".into(),
            ));
        }
        let filter = HistoryFilter::default();
        let mut items = Vec::new();
        loop {
            let page = store.history_list(
                &filter,
                Page {
                    offset: items.len() as u32,
                    limit: EXPORT_BATCH,
                },
            )?;
            let got = page.items.len();
            items.extend(page.items);
            if got == 0 || items.len() as u32 >= page.total {
                break;
            }
        }
        let bytes = match format {
            ExportFormat::Csv => history_csv(&items).into_bytes(),
            ExportFormat::Json => serde_json::to_vec_pretty(&items)
                .map_err(|e| Error::Invalid(format!("falha ao gerar JSON: {e}")))?,
        };
        write_atomic(path, &bytes)?;
        Ok(items.len() as u32)
    }

    pub fn history_clear(&self, before: Option<DateTime<Utc>>) -> Result<u32> {
        self.store()?.history_clear(before)
    }

    // --- Estatísticas ---

    pub fn stats_overview(&self, range: DateRange) -> Result<StatsOverview> {
        check_range(range)?;
        self.store()?.stats_overview(range)
    }

    pub fn stats_monthly(&self, range: DateRange) -> Result<Vec<MonthlyStat>> {
        check_range(range)?;
        self.store()?.stats_monthly(range)
    }

    // --- Ajustes e dados ---

    pub fn settings_get(&self) -> Result<Settings> {
        self.store()?.settings_get()
    }

    pub fn settings_set(&self, patch: &SettingsPatch) -> Result<Settings> {
        self.store()?.settings_set(patch)
    }

    pub fn backup_create(&self, path: &str, passphrase: &str) -> Result<()> {
        self.store()?.backup_create(Path::new(path), passphrase)
    }

    pub fn backup_restore(&self, path: &str, passphrase: &str) -> Result<()> {
        self.store()?.backup_restore(Path::new(path), passphrase)
    }

    /// "Apagar tudo": remove o banco e a chave, e abre um banco novo e vazio.
    pub fn data_wipe(&self) -> Result<()> {
        let mut guard = self.inner.write().unwrap_or_else(|p| p.into_inner());
        let Inner { config, services } = guard.take().ok_or_else(not_ready)?;
        let Services {
            store,
            gate,
            client,
            engine,
            policy,
        } = services;
        drop((engine, client, gate));
        let store = match Arc::try_unwrap(store) {
            Ok(store) => store,
            Err(store) => {
                // Ainda em uso (ciclo em andamento): remonta e recusa.
                *guard = Some(Inner {
                    services: Services::build_from(&config, store, policy),
                    config,
                });
                return Err(Error::Invalid(
                    "há uma operação em andamento; tente de novo em instantes".into(),
                ));
            }
        };
        store.wipe()?;
        config.keystore.delete()?;
        let key = keys::load_or_create(config.keystore.as_ref())?;
        let store = Store::open(&config.db_path(), &key)?;
        let services = Services::build(&config, store);
        *guard = Some(Inner { config, services });
        Ok(())
    }

    // --- Diagnóstico ---

    pub fn diagnostics_requests(&self, limit: u32) -> Result<Vec<RequestLogEntry>> {
        self.store()?.requests_recent(limit.clamp(1, 1000))
    }
}

/// Estado da automação: há conta automática ativa e a política remota permite.
pub fn automation_state(accounts: &[Account], policy: &RemotePolicy) -> AutomationState {
    let remote_enabled = policy.effective().automation_enabled;
    let has_auto = accounts
        .iter()
        .any(|a| a.mode == AccountMode::Auto && a.status == AccountStatus::Active);
    let paused_reason = if !remote_enabled {
        Some(
            policy
                .message
                .clone()
                .unwrap_or_else(|| "automação desligada pela política remota".into()),
        )
    } else if !has_auto {
        Some("nenhuma conta ativa no modo automático".into())
    } else {
        None
    };
    AutomationState {
        enabled: remote_enabled && has_auto,
        remote_enabled,
        paused_reason,
    }
}

/// Normaliza o endereço: só HTTPS e sem credenciais (regras do gate), sem
/// query/fragmento e sem barra final.
pub fn normalize_library_url(url: &str) -> Result<String> {
    let url = url.trim();
    validate_library_url(url)?;
    let end = url.find(['?', '#']).unwrap_or(url.len());
    Ok(url[..end].trim_end_matches('/').to_owned())
}

/// Regras do reagendamento ("só dentro da janela permitida"):
/// - só agendamentos pendentes ou pausados;
/// - no futuro e dentro da janela 07h–21h (America/Sao_Paulo);
/// - com empréstimo: no máximo no dia do vencimento;
/// - sem empréstimo: até [`MAX_RESCHEDULE_DAYS`] dias à frente.
pub fn validate_reschedule(
    schedule: &Schedule,
    loan_due: Option<chrono::NaiveDate>,
    run_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<()> {
    if !matches!(
        schedule.status,
        ScheduleStatus::Pending | ScheduleStatus::Paused
    ) {
        return Err(Error::Invalid(format!(
            "agendamento {} não pode ser reagendado",
            schedule.status.as_str()
        )));
    }
    if run_at <= now {
        return Err(Error::Invalid("o novo horário já passou".into()));
    }
    if !planner::in_window(run_at) {
        return Err(Error::Invalid(
            "o horário precisa estar entre 07h e 21h (Brasília)".into(),
        ));
    }
    match loan_due {
        Some(due) if planner::local_date(run_at) > due => Err(Error::Invalid(
            "o horário não pode passar do dia do vencimento".into(),
        )),
        None if run_at > now + Duration::days(MAX_RESCHEDULE_DAYS) => Err(Error::Invalid(format!(
            "só é possível reagendar até {MAX_RESCHEDULE_DAYS} dias à frente"
        ))),
        _ => Ok(()),
    }
}

fn check_range(range: DateRange) -> Result<()> {
    if range.from > range.to {
        return Err(Error::Invalid("período inválido".into()));
    }
    Ok(())
}

fn csv_field(s: &str) -> String {
    // Neutraliza injeção de fórmula em planilhas.
    let s = if s.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{s}")
    } else {
        s.to_owned()
    };
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s
    }
}

/// CSV (RFC 4180) do histórico.
pub fn history_csv(items: &[RenewalEvent]) -> String {
    let mut out = String::from(
        "id,loan_id,attempted_at,trigger,result,old_due_date,new_due_date,message\r\n",
    );
    for e in items {
        let row = [
            e.id.to_string(),
            e.loan_id.to_string(),
            e.attempted_at.to_rfc3339(),
            e.trigger.as_str().to_owned(),
            e.result.as_str().to_owned(),
            e.old_due_date.map(|d| d.to_string()).unwrap_or_default(),
            e.new_due_date.map(|d| d.to_string()).unwrap_or_default(),
            csv_field(e.message.as_deref().unwrap_or_default()),
        ];
        out.push_str(&row.join(","));
        out.push_str("\r\n");
    }
    out
}
