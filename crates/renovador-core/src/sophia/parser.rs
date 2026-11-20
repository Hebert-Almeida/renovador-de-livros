//! Parser das páginas do Sophia (W1-06). Estrutura desconhecida → `Error::Parse`.
//!
//! Os seletores vêm do app 1.0 (`main.js`, que funcionava no site real) e
//! ficam em `selectors.rs`. Datas e contadores são lidos do texto de cada
//! item, sem depender de classes CSS que o 1.0 não usava.

use chrono::NaiveDate;
use scraper::{ElementRef, Html, Selector};

use super::model::*;
use super::selectors;
use crate::{Error, Result};

/// Lê o JSON do login. O `url` vem entre aspas: remove um caractere de cada ponta.
pub fn parse_login(json: &str) -> Result<LoginResponse> {
    let body = json.trim_start_matches('\u{feff}').trim();
    let v: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| Error::Parse(format!("resposta de login não é JSON: {e}")))?;
    let obj = v
        .as_object()
        .ok_or_else(|| Error::Parse("resposta de login sem objeto".into()))?;
    let text = |k: &str| match obj.get(k) {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    };
    if !obj.contains_key("login_valido") {
        return Err(Error::Parse("resposta de login sem login_valido".into()));
    }
    Ok(LoginResponse {
        login_valido: text("login_valido"),
        mensagem: text("mensagem").trim().to_owned(),
        url: strip_quotes(&text("url")).to_owned(),
        abrir_popup: obj
            .get("abrir_popup")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    })
}

fn strip_quotes(s: &str) -> &str {
    let t = s.trim();
    for q in ['"', '\''] {
        if t.len() >= 2 && t.starts_with(q) && t.ends_with(q) {
            return t[1..t.len() - 1].trim();
        }
    }
    t
}

fn sel(css: &str) -> Selector {
    Selector::parse(css).expect("seletor CSS válido")
}

/// Texto do elemento com espaços normalizados.
fn text_of(el: ElementRef<'_>) -> String {
    el.text()
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>()
        .join(" ")
}

/// `true` se a página é o formulário de login (sessão expirada).
fn is_login_page(doc: &Html) -> bool {
    doc.select(&sel("input#login, input#senha, #botao_autenticar"))
        .next()
        .is_some()
}

/// Lê a lista de empréstimos (`div.ui-content > ul > li`).
///
/// Sem `div.ui-content`, ou com o formulário de login na página, é erro:
/// uma lista vazia por engano faria o app achar que todos os livros
/// foram devolvidos.
pub fn parse_loan_list(html: &str) -> Result<Vec<RemoteLoan>> {
    let doc = Html::parse_document(html);
    if is_login_page(&doc) {
        return Err(Error::Parse(
            "a lista voltou para a página de login (sessão expirada?)".into(),
        ));
    }
    if doc.select(&sel("div.ui-content")).next().is_none() {
        return Err(Error::Parse("página da lista sem div.ui-content".into()));
    }
    let title_sel = sel("div.div-textoLista > h3");
    let link_sel = sel("div.div-textoLista > div.ui-grid-a > div.ui-block-b > a");

    let mut loans = Vec::new();
    for li in doc.select(&sel(selectors::LOAN_ITEM)) {
        // Itens sem título são divisores ou avisos ("nenhum empréstimo").
        let Some(h3) = li.select(&title_sel).next() else {
            continue;
        };
        let title = text_of(h3);
        if title.is_empty() {
            continue;
        }
        let text = text_of(li);
        let due_date = due_date_in(&text).ok_or_else(|| {
            Error::Parse(format!("data de devolução não encontrada em \"{title}\""))
        })?;
        let renew_url = li
            .select(&link_sel)
            .next()
            .and_then(|a| a.value().attr("href"))
            .map(str::trim)
            .filter(|h| {
                let l = h.to_ascii_lowercase();
                !h.is_empty() && *h != "#" && !l.starts_with("javascript:")
            })
            .map(str::to_owned);
        loans.push(RemoteLoan {
            item_key: item_key(&text, &title),
            title,
            due_date,
            renewals_count: renewals_in(&text),
            renew_url,
        });
    }
    Ok(loans)
}

/// Todas as datas dd/mm/aaaa ou dd/mm/aa do texto, com a posição em que
/// começam. O Sophia mobile mostra "Data prevista: 23/11/26"; ano com dois
/// dígitos é lido como 20aa.
fn dates_in(text: &str) -> Vec<(usize, NaiveDate)> {
    let b = text.as_bytes();
    let digit = |j: usize| b.get(j).is_some_and(u8::is_ascii_digit);
    let mut out = Vec::new();
    let mut i = 0;
    while i + 8 <= b.len() {
        let head = (i == 0 || !b[i - 1].is_ascii_digit())
            && digit(i)
            && digit(i + 1)
            && b[i + 2] == b'/'
            && digit(i + 3)
            && digit(i + 4)
            && b[i + 5] == b'/'
            && digit(i + 6)
            && digit(i + 7);
        if head {
            let year_len = if digit(i + 8) && digit(i + 9) { 4 } else { 2 };
            let end = i + 6 + year_len;
            // Os bytes casados são ASCII, então o fatiamento cai em limites de caractere.
            if !digit(end)
                && let (Ok(day), Ok(month), Ok(year)) = (
                    text[i..i + 2].parse::<u32>(),
                    text[i + 3..i + 5].parse::<u32>(),
                    text[i + 6..end].parse::<i32>(),
                )
            {
                let year = if year_len == 2 { 2000 + year } else { year };
                if let Some(d) = NaiveDate::from_ymd_opt(year, month, day) {
                    out.push((i, d));
                    i = end;
                    continue;
                }
            }
        }
        i += 1;
    }
    out
}

/// Data de devolução: a primeira data depois de "devolu", "previs" ou
/// "venc"; sem rótulo, a última data do item (a do empréstimo vem antes).
fn due_date_in(text: &str) -> Option<NaiveDate> {
    let dates = dates_in(text);
    let lower = text.to_lowercase();
    for label in ["devolu", "previs", "venc"] {
        // Em pt-BR, `to_lowercase` mantém o tamanho em bytes: os índices batem.
        if let Some(pos) = lower.find(label)
            && let Some((_, d)) = dates.iter().find(|(p, _)| *p >= pos)
        {
            return Some(*d);
        }
    }
    dates.last().map(|(_, d)| *d)
}

/// Primeiro número logo depois de `label` (até 30 bytes adiante).
fn number_after(lower: &str, label: &str) -> Option<String> {
    let start = lower.find(label)? + label.len();
    let rest = lower.get(start..)?;
    let begin = rest
        .char_indices()
        .take_while(|(i, _)| *i <= 30)
        .find(|(_, c)| c.is_ascii_digit())?
        .0;
    let digits: String = rest[begin..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    // Não confunde com uma data (dd/mm/aa ou dd/mm/aaaa).
    if rest[begin + digits.len()..].starts_with('/') {
        return None;
    }
    Some(digits)
}

/// "Renovações: 2", "Nº de renovações 2" etc.
fn renewals_in(text: &str) -> Option<u32> {
    let lower = text.to_lowercase();
    ["renovações", "renovacoes", "renovação", "renovacao"]
        .iter()
        .find_map(|l| number_after(&lower, l))
        .and_then(|n| n.parse().ok())
}

/// Chave estável do exemplar: tombo/registro/código, se aparecer no texto;
/// senão, o título normalizado. Não usa o link de renovação, que pode mudar.
fn item_key(text: &str, title: &str) -> String {
    let lower = text.to_lowercase();
    for label in ["tombo", "registro", "exemplar", "código", "codigo"] {
        if let Some(n) = number_after(&lower, label) {
            return format!("{label}:{n}");
        }
    }
    format!("titulo:{}", title.to_lowercase())
}

/// Classifica a página devolvida pelo link de renovação.
///
/// Recusas conhecidas têm prioridade. Sem recusa, é sucesso; a engine
/// confirma relendo a lista, então um falso sucesso vira "não confirmada".
pub fn classify_renewal(html: &str) -> Result<RenewOutcome> {
    let doc = Html::parse_document(html);
    if is_login_page(&doc) {
        return Err(Error::Parse(
            "a renovação voltou para a página de login (sessão expirada?)".into(),
        ));
    }
    let root = doc
        .select(&sel("div.ui-content"))
        .next()
        .or_else(|| doc.select(&sel("body")).next())
        .unwrap_or_else(|| doc.root_element());
    let text = text_of(root);
    if text.is_empty() {
        return Err(Error::Parse("resposta de renovação vazia".into()));
    }
    let lower = text.to_lowercase();
    let message = summary(&text);
    let has = |words: &[&str]| words.iter().any(|w| lower.contains(w));

    let negative = has(&[
        "não foi possível",
        "nao foi possivel",
        "não é possível",
        "nao e possivel",
        "não pode",
        "nao pode",
        "não renov",
        "nao renov",
        "não foi renov",
        "nao foi renov",
        "não permitid",
        "nao permitid",
    ]);
    if !negative
        && has(&[
            "sucesso",
            "renovação efetuada",
            "renovacao efetuada",
            "renovação realizada",
            "renovacao realizada",
            "renovado",
            "renovada",
        ])
    {
        return Ok(RenewOutcome::Success {
            new_due_date: due_date_in(&text),
        });
    }
    if has(&["reservad", "possui reserva", "existe reserva", "há reserva"]) {
        return Ok(RenewOutcome::Reserved { message });
    }
    if has(&[
        "limite de renova",
        "máximo de renova",
        "maximo de renova",
        "excedeu",
        "excedido",
        "atingiu",
        "atingido",
    ]) {
        return Ok(RenewOutcome::LimitReached { message });
    }
    if has(&["atraso", "atrasad", "vencid"]) {
        return Ok(RenewOutcome::Overdue { message });
    }
    if has(&[
        "bloque", "suspens", "débito", "debito", "multa", "pendênc", "pendenc", "erro",
    ]) || negative
    {
        return Ok(RenewOutcome::Blocked { message });
    }
    Ok(RenewOutcome::Success {
        new_due_date: due_date_in(&text),
    })
}

/// Mensagem curta para o histórico (até 200 caracteres).
fn summary(text: &str) -> String {
    let mut s: String = text.chars().take(200).collect();
    if text.chars().count() > 200 {
        s.push('…');
    }
    s
}

/// `true` se a página pede CAPTCHA ou desafio do Cloudflare.
pub fn needs_human_verification(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    super::selectors::HUMAN_VERIFICATION_MARKERS
        .iter()
        .any(|m| lower.contains(&m.to_ascii_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn detects_cloudflare_challenge() {
        assert!(needs_human_verification("<title>Just a moment...</title>"));
        assert!(!needs_human_verification("<h3>Livro</h3>"));
    }

    #[test]
    fn login_ok_strips_quotes() {
        let r = parse_login(
            r#"{"login_valido":"true","mensagem":"","url":"'emprestimos.asp?codigo=1'","abrir_popup":0}"#,
        )
        .unwrap();
        assert_eq!(r.login_valido, "true");
        assert_eq!(r.url, "emprestimos.asp?codigo=1");
    }

    #[test]
    fn login_error_and_bool_value() {
        let r = parse_login(
            "\u{feff}{\"login_valido\":false,\"mensagem\":\" Senha inválida \",\"url\":\"\"}",
        )
        .unwrap();
        assert_eq!(r.login_valido, "false");
        assert_eq!(r.mensagem, "Senha inválida");
        assert!(parse_login("<html>").is_err());
        assert!(parse_login("{}").is_err());
    }

    const LIST: &str = r#"<html><body><div data-role="page"><div class="ui-content"><ul>
      <li data-role="list-divider">Empréstimos</li>
      <li><div class="div-textoLista"><h3>Dom  Casmurro</h3>
        <p>Tombo: 12345</p><p>Empréstimo: 03/11/2026</p>
        <p>Devolução prevista: 17/11/2026</p><p>Renovações: 2</p>
        <div class="ui-grid-a"><div class="ui-block-a"></div>
        <div class="ui-block-b"><a href="renovar.asp?codigo=9&amp;x=1">Renovar</a></div></div></div></li>
      <li><div class="div-textoLista"><h3>O Cortiço</h3>
        <p>Data: 01/11/2026</p><p>21/11/2026</p>
        <div class="ui-grid-a"><div class="ui-block-b"><a href="javascript:void(0)">Renovar</a></div></div></div></li>
    </ul></div></div></body></html>"#;

    #[test]
    fn parses_loan_list() {
        let l = parse_loan_list(LIST).unwrap();
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].title, "Dom Casmurro");
        assert_eq!(l[0].item_key, "tombo:12345");
        assert_eq!(l[0].due_date, d(2026, 11, 17));
        assert_eq!(l[0].renewals_count, Some(2));
        assert_eq!(l[0].renew_url.as_deref(), Some("renovar.asp?codigo=9&x=1"));
        assert_eq!(l[1].item_key, "titulo:o cortiço");
        assert_eq!(l[1].due_date, d(2026, 11, 21));
        assert_eq!(l[1].renewals_count, None);
        assert_eq!(l[1].renew_url, None);
    }

    #[test]
    fn empty_list_is_ok_but_login_page_is_error() {
        let empty = r#"<div class="ui-content"><ul><li>Nenhum empréstimo</li></ul></div>"#;
        assert!(parse_loan_list(empty).unwrap().is_empty());
        let login = r#"<div class="ui-content"><input id="login"><input id="senha"></div>"#;
        assert!(parse_loan_list(login).is_err());
        assert!(parse_loan_list("<html><body>?</body></html>").is_err());
    }

    #[test]
    fn missing_due_date_is_error() {
        let html = r#"<div class="ui-content"><ul><li><div class="div-textoLista"><h3>X</h3></div></li></ul></div>"#;
        assert!(parse_loan_list(html).is_err());
    }

    fn page(msg: &str) -> String {
        format!(r#"<html><body><div class="ui-content"><p>{msg}</p></div></body></html>"#)
    }

    #[test]
    fn classifies_renewal() {
        assert_eq!(
            classify_renewal(&page(
                "Renovação efetuada. Nova data de devolução: 24/11/2026"
            ))
            .unwrap(),
            RenewOutcome::Success {
                new_due_date: Some(d(2026, 11, 24))
            }
        );
        assert!(matches!(
            classify_renewal(&page("Exemplar reservado por outro usuário")).unwrap(),
            RenewOutcome::Reserved { .. }
        ));
        assert!(matches!(
            classify_renewal(&page("Limite de renovações atingido")).unwrap(),
            RenewOutcome::LimitReached { .. }
        ));
        assert!(matches!(
            classify_renewal(&page("Empréstimo em atraso")).unwrap(),
            RenewOutcome::Overdue { .. }
        ));
        assert!(matches!(
            classify_renewal(&page("Usuário bloqueado")).unwrap(),
            RenewOutcome::Blocked { .. }
        ));
        // "Data limite" numa página de sucesso não é recusa.
        assert_eq!(
            classify_renewal(&page("Livro renovado. Data limite: 01/12/2026")).unwrap(),
            RenewOutcome::Success {
                new_due_date: Some(d(2026, 12, 1))
            }
        );
        assert!(matches!(
            classify_renewal(&page("Não foi possível renovar: exemplar reservado")).unwrap(),
            RenewOutcome::Reserved { .. }
        ));
        assert!(classify_renewal(r#"<input id="login">"#).is_err());
    }

    /// Estrutura copiada de renovacoes.asp (Sophia mobile), com títulos,
    /// tombos e códigos trocados por valores fictícios.
    const MOBILE_LIST: &str = r#"<html><body>
<div data-role="content" class="ui-content" role="main">
<div id="logotiposophia-padrao"><img id="imglogotiposophia" alt="Sophia" src="logo.png"></div>
<ul data-role="listview" data-inset="true">    <li data-role="list-divider" role="heading">        Renovações    </li>    <li data-theme="d" class="ui-li-static ui-body-d">        <div class="div-textoLista"><h3>Livro de exemplo um</h3><p>Tombo: 000111</p><p>Biblioteca: <a class="link-mobile ui-link" href="https://exemplo.invalid/5362/mobile/detalhe_biblioteca.asp?codigo=1">Biblioteca Central</a></p><p id="data_prevista1">Data prevista: 23/11/26</p>        <div class="ui-grid-a">            <div class="ui-block-a"></div>            <div class="ui-block-b">	        <a href="https://exemplo.invalid/5362/mobile/renovar.asp?idioma=ptbr&amp;codigo_circulacao=1&amp;acesso=web" data-role="button" data-rel="dialog" class="botaoFechar ui-link ui-btn" role="button">Renovar</a>            </div>        </div>    </div></li>    <li data-theme="d" class="ui-li-static ui-body-d ui-last-child">        <div class="div-textoLista"><h3>Livro de exemplo dois</h3><p>Tombo: 222333</p><p>Biblioteca: <a class="link-mobile ui-link" href="https://exemplo.invalid/5362/mobile/detalhe_biblioteca.asp?codigo=1">Biblioteca Central</a></p><p id="data_prevista2">Data prevista: 02/01/27</p>        <div class="ui-grid-a">            <div class="ui-block-a"></div>            <div class="ui-block-b">	        <a href="https://exemplo.invalid/5362/mobile/renovar.asp?idioma=ptbr&amp;codigo_circulacao=2&amp;acesso=web" data-role="button" data-rel="dialog" class="botaoFechar ui-link ui-btn" role="button">Renovar</a>            </div>        </div>    </div></li></ul>
</div></body></html>"#;

    #[test]
    fn parses_mobile_list_with_two_digit_year() {
        let l = parse_loan_list(MOBILE_LIST).unwrap();
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].title, "Livro de exemplo um");
        assert_eq!(l[0].item_key, "tombo:000111");
        assert_eq!(l[0].due_date, d(2026, 11, 23));
        assert_eq!(l[0].renewals_count, None);
        assert_eq!(
            l[0].renew_url.as_deref(),
            Some(
                "https://exemplo.invalid/5362/mobile/renovar.asp?idioma=ptbr&codigo_circulacao=1&acesso=web"
            )
        );
        assert_eq!(l[1].item_key, "tombo:222333");
        assert_eq!(l[1].due_date, d(2027, 1, 2));
    }

    #[test]
    fn dates_accept_two_digit_year() {
        assert_eq!(
            dates_in("Data prevista: 23/11/26"),
            vec![(15, d(2026, 11, 23))]
        );
        assert_eq!(dates_in("23/11/261"), vec![]);
        assert_eq!(dates_in("31/02/26"), vec![]);
    }

    #[test]
    fn dates_ignore_longer_numbers() {
        assert_eq!(dates_in("123/11/2026 01/02/20261"), vec![]);
        assert_eq!(due_date_in("Vencimento 05/12/2026"), Some(d(2026, 12, 5)));
    }
}
