//! Notificações (W2-03) — textos da seção 5.4: renovado, falhou, vencendo,
//! senha inválida, automação pausada e livro que vence hoje sem renovação.

use chrono::NaiveDate;
use renovador_core::model::{RenewalEvent, RenewalResult};
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

pub fn renewed(title: &str, ev: &RenewalEvent) -> Note {
    let body = match ev.new_due_date {
        Some(d) => format!("“{title}” agora vence em {}.", br(d)),
        None => format!("“{title}” foi renovado."),
    };
    Note {
        title: "Livro renovado".into(),
        body,
    }
}

pub fn failed(title: &str, ev: &RenewalEvent) -> Note {
    let mut body = format!("“{title}”: {}.", reason(ev.result));
    if let Some(d) = ev.old_due_date {
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

/// `true` se o usuário deixou as notificações ligadas (padrão: ligadas).
fn enabled<R: Runtime>(app: &AppHandle<R>) -> bool {
    let state = app.state::<AppState>();
    let guard = state.store.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .as_ref()
        .and_then(|s| s.settings_get().ok())
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
    use chrono::Utc;
    use renovador_core::model::Trigger;

    fn ev(result: RenewalResult) -> RenewalEvent {
        RenewalEvent {
            id: 1,
            loan_id: 1,
            attempted_at: Utc::now(),
            trigger: Trigger::Auto,
            result,
            old_due_date: NaiveDate::from_ymd_opt(2026, 11, 16),
            new_due_date: NaiveDate::from_ymd_opt(2026, 11, 30),
            message: None,
        }
    }

    #[test]
    fn renewed_shows_new_date() {
        let n = renewed("Dom Casmurro", &ev(RenewalResult::Success));
        assert!(n.body.contains("30/11/2026"));
    }

    #[test]
    fn failed_shows_reason_and_return_date() {
        let n = failed("Dom Casmurro", &ev(RenewalResult::Reserved));
        assert!(n.body.contains("reservado"));
        assert!(n.body.contains("16/11/2026"));
    }
}
