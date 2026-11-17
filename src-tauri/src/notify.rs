//! Notificações (W2-03) — textos da seção 5.4: renovado, falhou, vencendo,
//! senha inválida, automação pausada e livro que vence hoje sem renovação.

use chrono::NaiveDate;
use renovador_core::engine::Notification;
use renovador_core::model::RenewalResult;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_notification::NotificationExt;

use crate::state::AppState;

/// Conteúdo de uma notificação, separado do envio para ser testável.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub title: String,
    pub body: String,
}

fn br(date: NaiveDate) -> String {
    date.format("%d/%m/%Y").to_string()
}

pub fn reason(result: RenewalResult) -> &'static str {
    match result {
        RenewalResult::Success => "renovado",
        RenewalResult::Reserved => "o livro está reservado por outra pessoa",
        RenewalResult::LimitReached => "o limite de renovações foi atingido",
        RenewalResult::Blocked => "a conta está bloqueada na biblioteca",
        RenewalResult::Overdue => "o empréstimo está atrasado",
        RenewalResult::NetworkError => "falha de rede",
        RenewalResult::ParseError => "a resposta da biblioteca não foi entendida",
        RenewalResult::Captcha => "a biblioteca pediu verificação humana",
        RenewalResult::RateLimited => "limite de acessos atingido",
    }
}

pub fn renewed(title: &str, new_due: Option<NaiveDate>) -> Note {
    let body = match new_due {
        Some(d) => format!("“{title}” agora vence em {}.", br(d)),
        None => format!("“{title}” foi renovado."),
    };
    Note {
        title: "Livro renovado".into(),
        body,
    }
}

pub fn failed(title: &str, result: RenewalResult, due: Option<NaiveDate>) -> Note {
    let mut body = format!("“{title}”: {}.", reason(result));
    if let Some(d) = due {
        body.push_str(&format!(" Devolva até {}.", br(d)));
    }
    Note {
        title: "Não foi possível renovar".into(),
        body,
    }
}

pub fn due_soon(title: &str, due: NaiveDate, today: NaiveDate) -> Note {
    let body = if due <= today {
        format!("“{title}” vence hoje ({}).", br(due))
    } else {
        format!("“{title}” vence em {}.", br(due))
    };
    Note {
        title: "Livro vencendo".into(),
        body,
    }
}

pub fn skipped_due_today(title: &str, due: NaiveDate) -> Note {
    Note {
        title: "Livro vence hoje".into(),
        body: format!(
            "“{title}” vence hoje ({}) e não foi renovado: o horário de renovação automática \
             (7h–21h) já passou. Devolva na biblioteca ou renove pelo site.",
            br(due)
        ),
    }
}

pub fn invalid_password(library: &str) -> Note {
    Note {
        title: "Senha inválida".into(),
        body: format!("A senha da conta em {library} não foi aceita. Atualize-a no app."),
    }
}

pub fn automation_paused(reason: Option<&str>) -> Note {
    Note {
        title: "Automação pausada".into(),
        body: match reason {
            Some(r) => format!("As renovações automáticas foram pausadas: {r}."),
            None => "As renovações automáticas foram pausadas.".into(),
        },
    }
}

/// Texto de uma notificação produzida pelo motor. `library` resolve o nome da
/// biblioteca de uma conta (para "senha inválida").
pub fn from_engine(
    n: &Notification,
    today: NaiveDate,
    library: impl Fn(renovador_core::model::Id) -> Option<String>,
) -> Note {
    match n {
        Notification::Renewed {
            title,
            new_due_date,
            ..
        } => renewed(title, *new_due_date),
        Notification::Failed {
            title,
            result,
            due_date,
            ..
        } => failed(title, *result, Some(*due_date)),
        Notification::DueToday {
            title, due_date, ..
        } => skipped_due_today(title, *due_date),
        Notification::Reminder {
            title, due_date, ..
        } => due_soon(title, *due_date, today),
        Notification::InvalidPassword { account_id } => {
            invalid_password(&library(*account_id).unwrap_or_else(|| "sua biblioteca".into()))
        }
        Notification::AutomationPaused { reason, .. } => automation_paused(Some(reason)),
    }
}

/// `true` se o usuário deixou as notificações ligadas (padrão: ligadas).
fn enabled<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.state::<AppState>()
        .with_store(|s| s.settings_get().ok())
        .flatten()
        .is_none_or(|s| s.notifications)
}

pub fn send<R: Runtime>(app: &AppHandle<R>, note: Note) {
    if !enabled(app) {
        return;
    }
    if let Err(e) = app
        .notification()
        .builder()
        .title(note.title)
        .body(note.body)
        .show()
    {
        tracing::warn!("falha ao notificar: {e}");
    }
}

pub fn init(app: &mut tauri::App) -> tauri::Result<()> {
    use tauri_plugin_notification::PermissionState;
    let n = app.notification();
    if !matches!(n.permission_state(), Ok(PermissionState::Granted)) {
        let _ = n.request_permission();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
    }

    #[test]
    fn renewed_shows_new_date() {
        let n = renewed("Dom Casmurro", Some(d(30)));
        assert!(n.body.contains("30/11/2026"));
    }

    #[test]
    fn failed_shows_reason_and_return_date() {
        let n = failed("Dom Casmurro", RenewalResult::Reserved, Some(d(16)));
        assert!(n.body.contains("reservado"));
        assert!(n.body.contains("16/11/2026"));
    }

    #[test]
    fn engine_due_today_becomes_skipped_note() {
        let n = from_engine(
            &Notification::DueToday {
                loan_id: 1,
                title: "Dom Casmurro".into(),
                due_date: d(16),
            },
            d(16),
            |_| None,
        );
        assert_eq!(n, skipped_due_today("Dom Casmurro", d(16)));
    }

    #[test]
    fn engine_invalid_password_names_the_library() {
        let n = from_engine(
            &Notification::InvalidPassword { account_id: 7 },
            d(16),
            |id| (id == 7).then(|| "Biblioteca Central".into()),
        );
        assert!(n.body.contains("Biblioteca Central"));
    }
}
