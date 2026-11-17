//! Loop de agendamento no desktop (W2-03): acorda a cada 30 min, com catch-up.
//!
//! A decisão do que fazer em cada tique é pura ([`Ticker`] e [`plan_tick`]) e
//! testada com [`FakeClock`](renovador_core::clock::FakeClock). Só há rede quando
//! existe tarefa vencida; o ciclo em si é do motor (W2-01), chamado por
//! `AppState::run_cycle` com o gatilho real (auto/catch-up não gastam a cota manual).

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, Duration, NaiveDate, Timelike, Utc};
use renovador_core::model::{CycleKind, Id, Schedule, ScheduleKind, ScheduleStatus, Trigger};
use renovador_core::planner::{in_window, local_date};
use tokio::sync::Notify;

/// Intervalo do loop (5.4).
pub const TICK: Duration = Duration::minutes(30);
/// Hora local (Brasília) em que a janela de renovação fecha.
const WINDOW_END_HOUR: u32 = 21;

/// Controle compartilhado com a bandeja.
#[derive(Default)]
pub struct SchedulerControl {
    paused: AtomicBool,
    wake: Notify,
}

impl SchedulerControl {
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }
    pub fn set_paused(&self, on: bool) {
        self.paused.store(on, Ordering::SeqCst);
        self.wake.notify_one();
    }
}

/// Relógio do loop: o primeiro tique sai imediatamente (catch-up ao iniciar) e os
/// seguintes a cada [`TICK`]. Um intervalo muito maior que o esperado (PC suspenso)
/// também é tratado como catch-up.
#[derive(Debug, Default)]
pub struct Ticker {
    last: Option<DateTime<Utc>>,
}

impl Ticker {
    /// `Some(trigger)` se é hora de um tique em `now`.
    pub fn poll(&mut self, now: DateTime<Utc>) -> Option<Trigger> {
        let trigger = match self.last {
            None => Trigger::CatchUp,
            Some(last) if now - last >= TICK * 2 => Trigger::CatchUp,
            Some(last) if now - last >= TICK => Trigger::Auto,
            Some(_) => return None,
        };
        self.last = Some(now);
        Some(trigger)
    }
}

/// O que um tique precisa fazer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Job {
    /// Rodar um ciclo da conta e fechar os agendamentos listados.
    Cycle {
        account_id: Id,
        kind: CycleKind,
        schedule_ids: Vec<Id>,
    },
    /// Lembrete D−2/D0.
    Remind { schedule_id: Id, loan_id: Id },
    /// Renovação de livro que vence hoje perdeu a janela (app aberto após 21h).
    SkippedDueToday { schedule_id: Id, loan_id: Id },
}

/// Decide os trabalhos de um tique a partir dos agendamentos pendentes.
/// `due_date` devolve o vencimento do empréstimo, se conhecido.
pub fn plan_tick(
    pending: &[Schedule],
    due_date: impl Fn(Id) -> Option<NaiveDate>,
    now: DateTime<Utc>,
) -> Vec<Job> {
    let today = local_date(now);
    let after_window = now.with_timezone(&chrono_tz_sp()).hour() >= WINDOW_END_HOUR;
    let open = in_window(now);
    let mut jobs = Vec::new();
    let mut cycles: BTreeMap<Id, (CycleKind, Vec<Id>)> = BTreeMap::new();

    for s in pending
        .iter()
        .filter(|s| s.status == ScheduleStatus::Pending && s.run_at <= now)
    {
        match s.kind {
            ScheduleKind::Remind => {
                if let Some(loan_id) = s.loan_id {
                    jobs.push(Job::Remind {
                        schedule_id: s.id,
                        loan_id,
                    });
                }
            }
            ScheduleKind::Renew | ScheduleKind::Sync if open => {
                let entry = cycles
                    .entry(s.account_id)
                    .or_insert((CycleKind::Sync, Vec::new()));
                if s.kind == ScheduleKind::Renew {
                    entry.0 = CycleKind::RenewDue;
                }
                entry.1.push(s.id);
            }
            ScheduleKind::Renew => {
                // Fora da janela: espera a próxima, exceto se o livro vence hoje e a
                // janela de hoje já fechou — aí só resta avisar.
                if let Some(loan_id) = s.loan_id
                    && after_window
                    && due_date(loan_id).is_some_and(|d| d <= today)
                {
                    jobs.push(Job::SkippedDueToday {
                        schedule_id: s.id,
                        loan_id,
                    });
                }
            }
            ScheduleKind::Sync => {}
        }
    }

    jobs.extend(
        cycles
            .into_iter()
            .map(|(account_id, (kind, schedule_ids))| Job::Cycle {
                account_id,
                kind,
                schedule_ids,
            }),
    );
    jobs
}

fn chrono_tz_sp() -> chrono::FixedOffset {
    // Brasília não tem horário de verão desde 2019.
    chrono::FixedOffset::west_opt(3 * 3600).expect("offset válido")
}

#[cfg(desktop)]
pub use desktop::{start, sync_now};

#[cfg(not(desktop))]
pub fn start(_app: tauri::AppHandle) {}

#[cfg(desktop)]
mod desktop {
    use std::sync::Arc;

    use renovador_core::clock::{Clock, SystemClock};
    use renovador_core::engine::Notification;
    use renovador_core::model::{AccountStatus, ScheduleFilter};
    use renovador_core::planner::local_date;
    use tauri::{AppHandle, Manager};
    use tauri_specta::Event;

    use super::*;
    use crate::events::{AutomationStateEvent, CycleFinishedEvent};
    use crate::notify;
    use crate::state::AppState;

    /// Frequência com que o relógio de parede é conferido (pega a volta da suspensão).
    const POLL: std::time::Duration = std::time::Duration::from_secs(60);

    pub fn start(app: AppHandle) {
        let control = Arc::new(SchedulerControl::default());
        // A pausa da bandeja sobrevive a reinícios (Ajustes: automation_paused).
        if with_store(&app, |s| s.settings_get().ok())
            .flatten()
            .is_some_and(|s| s.automation_paused)
        {
            control.set_paused(true);
        }
        app.manage(control);
        tauri::async_runtime::spawn(run_loop(app, Arc::new(SystemClock)));
    }

    async fn run_loop(app: AppHandle, clock: Arc<dyn Clock>) {
        let control = app.state::<Arc<SchedulerControl>>().inner().clone();
        let mut ticker = Ticker::default();
        loop {
            let now = clock.now();
            if !control.is_paused()
                && let Some(trigger) = ticker.poll(now)
            {
                sync_autostart(&app);
                let jobs = pending_jobs(&app, now);
                run_jobs(&app, jobs, trigger).await;
            }
            tokio::select! {
                _ = tokio::time::sleep(POLL) => {}
                _ = control.wake.notified() => {}
            }
        }
    }

    fn with_store<T>(
        app: &AppHandle,
        f: impl FnOnce(&renovador_core::store::Store) -> T,
    ) -> Option<T> {
        app.state::<AppState>().with_store(f)
    }

    fn pending_jobs(app: &AppHandle, now: DateTime<Utc>) -> Vec<Job> {
        with_store(app, |store| {
            let filter = ScheduleFilter {
                status: Some(ScheduleStatus::Pending),
                ..Default::default()
            };
            let pending = match store.schedule_list(&filter) {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!("agenda indisponível: {e}");
                    return Vec::new();
                }
            };
            plan_tick(
                &pending,
                |loan_id| store.loan_get(loan_id).ok().map(|d| d.view.loan.due_date),
                now,
            )
        })
        .unwrap_or_default()
    }

    fn sync_autostart(app: &AppHandle) {
        use tauri_plugin_autostart::ManagerExt;
        let Some(Ok(settings)) = with_store(app, |s| s.settings_get()) else {
            return;
        };
        let launcher = app.autolaunch();
        if launcher.is_enabled().ok() != Some(settings.autostart) {
            let r = if settings.autostart {
                launcher.enable()
            } else {
                launcher.disable()
            };
            if let Err(e) = r {
                tracing::warn!("autostart: {e}");
            }
        }
    }

    fn loan_title(app: &AppHandle, loan_id: Id) -> Option<(String, NaiveDate)> {
        with_store(app, |s| s.loan_get(loan_id).ok())
            .flatten()
            .map(|d| (d.view.book.title, d.view.loan.due_date))
    }

    fn close_schedule(app: &AppHandle, id: Id, status: ScheduleStatus) {
        use renovador_core::model::ScheduleOrigin;
        let r = with_store(app, |s| {
            s.schedule_set_status(id, status, ScheduleOrigin::System)
        });
        if let Some(Err(e)) = r {
            tracing::warn!("agendamento {id}: {e}");
        }
    }

    /// Executa os trabalhos de um tique (ou de "Sincronizar agora").
    pub async fn run_jobs(app: &AppHandle, jobs: Vec<Job>, trigger: Trigger) {
        for job in jobs {
            match job {
                Job::Remind {
                    schedule_id,
                    loan_id,
                } => {
                    if let Some((title, due)) = loan_title(app, loan_id) {
                        let today = local_date(Utc::now());
                        notify::send(app, notify::due_soon(&title, due, today));
                    }
                    close_schedule(app, schedule_id, ScheduleStatus::Done);
                }
                Job::SkippedDueToday {
                    schedule_id,
                    loan_id,
                } => {
                    if let Some((title, due)) = loan_title(app, loan_id) {
                        notify::send(app, notify::skipped_due_today(&title, due));
                    }
                    close_schedule(app, schedule_id, ScheduleStatus::Failed);
                }
                Job::Cycle {
                    account_id,
                    kind,
                    schedule_ids,
                } => {
                    let result = app
                        .state::<AppState>()
                        .run_cycle(account_id, kind, trigger)
                        .await;
                    match result {
                        // O motor já fecha os agendamentos que tratou e replaneja a agenda.
                        Ok(outcome) => {
                            let _ = CycleFinishedEvent(outcome.report.clone()).emit(app);
                            notify_outcome(app, &outcome.notifications);
                        }
                        Err(e) => {
                            tracing::warn!("ciclo da conta {account_id}: {e}");
                            for id in schedule_ids {
                                close_schedule(app, id, ScheduleStatus::Failed);
                            }
                        }
                    }
                }
            }
        }
    }

    fn notify_outcome(app: &AppHandle, notifications: &[Notification]) {
        let today = local_date(Utc::now());
        let library = |id: Id| {
            with_store(app, |s| s.account_get(id).ok())
                .flatten()
                .map(|a| a.library_name)
        };
        for n in notifications {
            notify::send(app, notify::from_engine(n, today, library));
        }
    }

    /// "Sincronizar agora" da bandeja: um ciclo de sincronização por conta ativa.
    pub fn sync_now(app: &AppHandle) {
        let accounts = with_store(app, |s| s.account_list().ok())
            .flatten()
            .unwrap_or_default();
        let jobs = accounts
            .into_iter()
            .filter(|a| a.status == AccountStatus::Active)
            .map(|a| Job::Cycle {
                account_id: a.id,
                kind: CycleKind::Sync,
                schedule_ids: Vec::new(),
            })
            .collect();
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            run_jobs(&app, jobs, Trigger::Manual).await;
        });
    }

    /// Alterna a pausa da automação, avisa o frontend e notifica.
    pub fn toggle_pause(app: &AppHandle) -> bool {
        let control = app.state::<Arc<SchedulerControl>>();
        let paused = !control.is_paused();
        control.set_paused(paused);
        let patch = renovador_core::model::SettingsPatch {
            automation_paused: Some(paused),
            ..Default::default()
        };
        if let Some(Err(e)) = with_store(app, |s| s.settings_set(&patch)) {
            tracing::warn!("não foi possível salvar a pausa: {e}");
        }
        let state = renovador_core::model::AutomationState {
            enabled: !paused,
            remote_enabled: true,
            paused_reason: paused.then(|| "pausada pela bandeja".to_owned()),
        };
        if let Err(e) = AutomationStateEvent(state).emit(app) {
            tracing::warn!("evento de automação: {e}");
        }
        if paused {
            notify::send(app, notify::automation_paused(Some("pausada pela bandeja")));
        }
        paused
    }
}

#[cfg(desktop)]
pub use desktop::toggle_pause;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use renovador_core::clock::{Clock, FakeClock};
    use renovador_core::model::ScheduleOrigin;

    /// Instante UTC a partir de hora local de Brasília (UTC−3).
    fn sp(d: u32, h: u32, m: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, d, h, m, 0).unwrap() + Duration::hours(3)
    }

    fn sched(id: Id, account_id: Id, kind: ScheduleKind, run_at: DateTime<Utc>) -> Schedule {
        Schedule {
            id,
            account_id,
            loan_id: Some(id * 10),
            kind,
            run_at,
            status: ScheduleStatus::Pending,
            origin: ScheduleOrigin::System,
            updated_at: run_at,
        }
    }

    #[test]
    fn ticker_catches_up_then_waits_30_min() {
        let clock = FakeClock::new(sp(16, 9, 0));
        let mut t = Ticker::default();
        assert_eq!(t.poll(clock.now()), Some(Trigger::CatchUp));
        clock.advance(Duration::minutes(29));
        assert_eq!(t.poll(clock.now()), None);
        clock.advance(Duration::minutes(1));
        assert_eq!(t.poll(clock.now()), Some(Trigger::Auto));
    }

    #[test]
    fn ticker_treats_long_sleep_as_catch_up() {
        let clock = FakeClock::new(sp(16, 9, 0));
        let mut t = Ticker::default();
        t.poll(clock.now());
        clock.advance(Duration::hours(8));
        assert_eq!(t.poll(clock.now()), Some(Trigger::CatchUp));
    }

    #[test]
    fn no_due_task_means_no_job() {
        let now = sp(16, 9, 0);
        let s = [sched(1, 1, ScheduleKind::Renew, now + Duration::hours(1))];
        assert!(plan_tick(&s, |_| None, now).is_empty());
    }

    #[test]
    fn overdue_tasks_grouped_per_account_in_window() {
        let clock = FakeClock::new(sp(16, 7, 30));
        let s = [
            sched(1, 1, ScheduleKind::Sync, sp(15, 10, 0)),
            sched(2, 1, ScheduleKind::Renew, sp(16, 7, 10)),
            sched(3, 2, ScheduleKind::Sync, sp(16, 7, 0)),
        ];
        let jobs = plan_tick(&s, |_| None, clock.now());
        assert_eq!(
            jobs,
            vec![
                Job::Cycle {
                    account_id: 1,
                    kind: CycleKind::RenewDue,
                    schedule_ids: vec![1, 2]
                },
                Job::Cycle {
                    account_id: 2,
                    kind: CycleKind::Sync,
                    schedule_ids: vec![3]
                },
            ]
        );
    }

    #[test]
    fn outside_window_waits_but_reminds() {
        let now = sp(16, 5, 0);
        let s = [
            sched(1, 1, ScheduleKind::Renew, sp(15, 20, 0)),
            sched(2, 1, ScheduleKind::Remind, sp(15, 20, 0)),
        ];
        let due = NaiveDate::from_ymd_opt(2026, 11, 17);
        assert_eq!(
            plan_tick(&s, |_| due, now),
            vec![Job::Remind {
                schedule_id: 2,
                loan_id: 20
            }]
        );
    }

    #[test]
    fn due_today_after_21h_is_reported_as_skipped() {
        let now = sp(16, 22, 15);
        let s = [sched(1, 1, ScheduleKind::Renew, sp(16, 14, 0))];
        let today = NaiveDate::from_ymd_opt(2026, 11, 16);
        assert_eq!(
            plan_tick(&s, |_| today, now),
            vec![Job::SkippedDueToday {
                schedule_id: 1,
                loan_id: 10
            }]
        );
        // Vencendo amanhã: só espera a janela de amanhã.
        let tomorrow = NaiveDate::from_ymd_opt(2026, 11, 17);
        assert!(plan_tick(&s, |_| tomorrow, now).is_empty());
    }

    #[test]
    fn loop_over_a_day_with_fake_clock() {
        // Simula 24 h de tiques: a renovação sai no primeiro tique dentro da janela.
        let clock = FakeClock::new(sp(16, 0, 0));
        let mut t = Ticker::default();
        let s = [sched(1, 1, ScheduleKind::Renew, sp(15, 23, 0))];
        let mut first_cycle = None;
        for _ in 0..(24 * 60) {
            if t.poll(clock.now()).is_some()
                && first_cycle.is_none()
                && !plan_tick(&s, |_| None, clock.now()).is_empty()
            {
                first_cycle = Some(clock.now());
            }
            clock.advance(Duration::minutes(1));
        }
        assert_eq!(first_cycle, Some(sp(16, 7, 0)));
    }

    #[test]
    fn paused_control_flag() {
        let c = SchedulerControl::default();
        assert!(!c.is_paused());
        c.set_paused(true);
        assert!(c.is_paused());
    }
}
