//! Cliente HTTP do Sophia (seção 5.1). Cada requisição passa pelo
//! [`RequestGate`]: `acquire` antes, `report` depois.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::header::{ACCEPT_LANGUAGE, CONTENT_TYPE, HeaderMap, HeaderValue};
use url::Url;

use super::detect::{self, Detection};
use super::{
    HttpResponse, LoggedSession, LoginResponse, LoginResult, RemoteLoan, RenewOutcome,
    SophiaClient, Transport, parser, selectors, user_agent,
};
use crate::guard::{GuardError, Permit, RequestGate, RequestOutcome, SessionTicket};
use crate::model::Trigger;
use crate::{Error, Result};

/// Timeout de cada requisição (seção 5.1).
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Transporte de produção com reqwest: cookies, gzip, timeout de 15 s,
/// User-Agent honesto e `Accept-Language: pt-BR`.
pub struct ReqwestTransport {
    http: reqwest::Client,
}

impl ReqwestTransport {
    pub fn new() -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("pt-BR"));
        let http = reqwest::Client::builder()
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                let origin = attempt.previous().first().and_then(|u| u.host_str());
                if attempt.previous().len() >= 10 || attempt.url().host_str() != origin {
                    attempt.stop()
                } else {
                    attempt.follow()
                }
            }))
            .gzip(true)
            .timeout(REQUEST_TIMEOUT)
            .user_agent(user_agent())
            .default_headers(headers)
            .build()
            .map_err(|e| Error::Network(e.to_string()))?;
        Ok(Self { http })
    }

    async fn into_response(resp: reqwest::Response) -> Result<HttpResponse> {
        let status = resp.status().as_u16();
        let final_url = resp.url().to_string();
        let headers = resp
            .headers()
            .iter()
            .filter_map(|(k, v)| Some((k.as_str().to_owned(), v.to_str().ok()?.to_owned())))
            .collect();
        let body = resp
            .text()
            .await
            .map_err(|e| Error::Network(e.to_string()))?;
        Ok(HttpResponse {
            status,
            final_url,
            headers,
            body,
        })
    }
}

#[async_trait]
impl Transport for ReqwestTransport {
    async fn get(&self, url: &str) -> Result<HttpResponse> {
        let resp = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| Error::Network(e.to_string()))?;
        Self::into_response(resp).await
    }

    async fn post_form(&self, url: &str, form: &[(&str, &str)]) -> Result<HttpResponse> {
        let body = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(form)
            .finish();
        let resp = self
            .http
            .post(url)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .map_err(|e| Error::Network(e.to_string()))?;
        Self::into_response(resp).await
    }
}

/// Funções de parsing usadas pelo cliente. O padrão são as de `parser.rs`;
/// testes podem trocá-las enquanto o parser real (W1-06) não existe.
#[derive(Clone, Copy)]
pub struct Parsers {
    pub login: fn(&str) -> Result<LoginResponse>,
    pub loan_list: fn(&str) -> Result<Vec<RemoteLoan>>,
    pub renewal: fn(&str) -> Result<RenewOutcome>,
}

impl Default for Parsers {
    fn default() -> Self {
        Self {
            login: parser::parse_login,
            loan_list: parser::parse_loan_list,
            renewal: parser::classify_renewal,
        }
    }
}

/// Configuração do cliente.
#[derive(Clone)]
pub struct ClientConfig {
    /// Exige HTTPS no endereço da biblioteca. Só os testes (wiremock) desligam.
    pub require_https: bool,
    /// Caminho de logout relativo a `{base}/mobile/`, se o site tiver um.
    pub logout_path: Option<String>,
    pub parsers: Parsers,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            require_https: true,
            logout_path: None,
            parsers: Parsers::default(),
        }
    }
}

#[derive(Clone, Copy)]
enum Request<'a> {
    /// GET idempotente: até 1 nova tentativa por erro de rede.
    Get(&'a str),
    /// GET com efeito colateral (renovação): sem nova tentativa.
    GetOnce(&'a str),
    Post(&'a str, &'a [(&'a str, &'a str)]),
}

/// Implementação de [`SophiaClient`] sobre um [`Transport`] e um [`RequestGate`].
pub struct HttpSophiaClient {
    transport: Arc<dyn Transport>,
    gate: Arc<dyn RequestGate>,
    config: ClientConfig,
}

impl HttpSophiaClient {
    pub fn new(
        transport: Arc<dyn Transport>,
        gate: Arc<dyn RequestGate>,
        config: ClientConfig,
    ) -> Self {
        Self {
            transport,
            gate,
            config,
        }
    }

    /// Cliente de produção com [`ReqwestTransport`].
    pub fn with_reqwest(gate: Arc<dyn RequestGate>) -> Result<Self> {
        Ok(Self::new(
            Arc::new(ReqwestTransport::new()?),
            gate,
            ClientConfig::default(),
        ))
    }

    /// Normaliza o endereço da biblioteca, garantindo a barra final.
    fn base(&self, base_url: &str) -> Result<Url> {
        let mut url =
            Url::parse(base_url.trim()).map_err(|e| Error::InvalidLibrary(e.to_string()))?;
        match url.scheme() {
            "https" => {}
            "http" if !self.config.require_https => {}
            _ => {
                return Err(Error::InvalidLibrary(
                    "o endereço precisa usar HTTPS".into(),
                ));
            }
        }
        if url.host_str().is_none() {
            return Err(Error::InvalidLibrary("endereço sem host".into()));
        }
        if !url.path().ends_with('/') {
            let path = format!("{}/", url.path());
            url.set_path(&path);
        }
        url.set_query(None);
        url.set_fragment(None);
        Ok(url)
    }

    /// Monta `{base}/mobile/<path>` e devolve também o host esperado.
    fn mobile_url(&self, base_url: &str, path: &str) -> Result<(String, String)> {
        self.resolve(base_url, "mobile/", path)
    }

    /// Monta `{base}/<path>` (caminhos de `selectors`, que já trazem `mobile/`).
    fn site_url(&self, base_url: &str, path: &str) -> Result<(String, String)> {
        self.resolve(base_url, "", path)
    }

    fn resolve(&self, base_url: &str, dir: &str, path: &str) -> Result<(String, String)> {
        let base = self.base(base_url)?;
        let dir_url = base
            .join(dir)
            .map_err(|e| Error::InvalidLibrary(e.to_string()))?;
        let url = dir_url
            .join(path.trim_start_matches('/'))
            .map_err(|e| Error::Parse(format!("link inválido: {e}")))?;
        let host = detect::host_of(base.as_str()).unwrap_or_default();
        // Links vindos da página nunca podem levar a outro host.
        if detect::host_of(url.as_str()).as_deref() != Some(host.as_str()) {
            return Err(Error::InvalidLibrary(format!(
                "link para outro host: {url}"
            )));
        }
        Ok((url.to_string(), host))
    }

    /// Faz uma requisição pelo gate e aplica a detecção. Em caso de sucesso,
    /// devolve a permissão para o chamador reportar após o parsing.
    async fn send(
        &self,
        ticket: &SessionTicket,
        kind: &str,
        host: &str,
        req: Request<'_>,
    ) -> Result<(Permit, HttpResponse)> {
        let attempts = if matches!(req, Request::Get(_)) { 2 } else { 1 };
        let mut last_err = None;
        for _ in 0..attempts {
            let permit = self.gate.acquire(ticket, kind).await?;
            let result = match req {
                Request::Get(url) | Request::GetOnce(url) => self.transport.get(url).await,
                Request::Post(url, form) => self.transport.post_form(url, form).await,
            };
            let resp = match result {
                Ok(r) => r,
                Err(e) => {
                    self.gate
                        .report(permit, None, RequestOutcome::NetworkError, None);
                    last_err = Some(e);
                    continue;
                }
            };
            let status = Some(resp.status);
            return match detect::inspect(host, &resp) {
                None => Ok((permit, resp)),
                Some(Detection::HumanVerification) => {
                    self.gate
                        .report(permit, status, RequestOutcome::HumanVerification, None);
                    Err(Error::HumanVerificationRequired)
                }
                Some(Detection::ForeignHost { host }) => {
                    self.gate
                        .report(permit, status, RequestOutcome::UnexpectedHtml, None);
                    Err(Error::InvalidLibrary(format!(
                        "a resposta veio de outro host: {host}"
                    )))
                }
                Some(Detection::RateLimited { retry_after }) => {
                    self.gate
                        .report(permit, status, RequestOutcome::RateLimited, retry_after);
                    Err(Error::Network(format!(
                        "o servidor pediu para aguardar (HTTP {})",
                        resp.status
                    )))
                }
                Some(Detection::ServerError) => {
                    self.gate
                        .report(permit, status, RequestOutcome::ServerError, None);
                    Err(Error::Network(format!(
                        "erro do servidor (HTTP {})",
                        resp.status
                    )))
                }
            };
        }
        Err(last_err.unwrap_or_else(|| Error::Network("falha de rede".into())))
    }

    /// Reporta o desfecho do parsing (Ok ou HTML inesperado).
    fn finish<T>(&self, permit: Permit, resp: &HttpResponse, parsed: Result<T>) -> Result<T> {
        let outcome = if parsed.is_ok() {
            RequestOutcome::Ok
        } else {
            RequestOutcome::UnexpectedHtml
        };
        self.gate.report(permit, Some(resp.status), outcome, None);
        parsed
    }

    fn ok_status(resp: &HttpResponse) -> Result<()> {
        if (200..300).contains(&resp.status) {
            Ok(())
        } else {
            Err(Error::Parse(format!("HTTP {} inesperado", resp.status)))
        }
    }
}

#[async_trait]
impl SophiaClient for HttpSophiaClient {
    async fn validate_library(&self, base_url: &str) -> Result<String> {
        let (url, host) = self.site_url(base_url, selectors::LOGIN_PAGE)?;
        // Ainda não há conta: sessão própria no gate com id 0 ("nenhuma conta").
        let ticket = self
            .gate
            .begin_session(0, base_url, Trigger::Manual)
            .await?;
        let result = async {
            let (permit, resp) = self
                .send(&ticket, "validate", &host, Request::Get(&url))
                .await?;
            let checked = Self::ok_status(&resp).and_then(|()| {
                if detect::is_sophia_site(&resp) {
                    Ok(())
                } else {
                    Err(Error::InvalidLibrary(
                        "o servidor não se identifica como Sophia".into(),
                    ))
                }
            });
            self.finish(permit, &resp, checked)?;
            Ok(host.clone())
        }
        .await;
        self.gate.end_session(ticket);
        result
    }

    async fn login(
        &self,
        ticket: &SessionTicket,
        base_url: &str,
        login: &str,
        password: &str,
    ) -> Result<LoginResult> {
        let (page_url, host) = self.site_url(base_url, selectors::LOGIN_PAGE)?;
        if !ticket.host.eq_ignore_ascii_case(&host) {
            return Err(Error::Guard(GuardError::HostNotAllowed { host }));
        }
        // 1. Página de login: define os cookies de sessão.
        let (permit, resp) = self
            .send(ticket, "login_page", &host, Request::Get(&page_url))
            .await?;
        self.finish(permit, &resp, Self::ok_status(&resp))?;

        // 2. POST do login (não idempotente: sem nova tentativa).
        let (post_url, _) = self.site_url(base_url, selectors::LOGIN_POST)?;
        let form = selectors::login_form(login, password);
        let (permit, resp) = self
            .send(ticket, "login", &host, Request::Post(&post_url, &form))
            .await?;
        let parsed = Self::ok_status(&resp).and_then(|()| (self.config.parsers.login)(&resp.body));
        let parsed = match parsed {
            Ok(p) => p,
            Err(e) => return self.finish(permit, &resp, Err(e)),
        };
        if parsed.login_valido.trim().eq_ignore_ascii_case("true") {
            if parsed.url.trim().is_empty() {
                return self.finish(
                    permit,
                    &resp,
                    Err(Error::Parse("login sem url da lista".into())),
                );
            }
            self.gate
                .report(permit, Some(resp.status), RequestOutcome::Ok, None);
            Ok(LoginResult::Ok(LoggedSession {
                base_url: self.base(base_url)?.to_string(),
                loans_path: parsed.url,
            }))
        } else {
            self.gate.report(
                permit,
                Some(resp.status),
                RequestOutcome::InvalidCredentials,
                None,
            );
            Ok(LoginResult::InvalidCredentials {
                message: parsed.mensagem,
            })
        }
    }

    async fn list_loans(
        &self,
        ticket: &SessionTicket,
        session: &LoggedSession,
    ) -> Result<Vec<RemoteLoan>> {
        let (url, host) = self.mobile_url(&session.base_url, &session.loans_path)?;
        let (permit, resp) = self.send(ticket, "list", &host, Request::Get(&url)).await?;
        let parsed =
            Self::ok_status(&resp).and_then(|()| (self.config.parsers.loan_list)(&resp.body));
        self.finish(permit, &resp, parsed)
    }

    async fn renew(
        &self,
        ticket: &SessionTicket,
        session: &LoggedSession,
        loan: &RemoteLoan,
    ) -> Result<RenewOutcome> {
        let link = loan
            .renew_url
            .as_deref()
            .ok_or_else(|| Error::Invalid("empréstimo sem link de renovação".into()))?;
        let (url, host) = self.mobile_url(&session.base_url, link)?;
        // A renovação é um GET no site, mas tem efeito: sem nova tentativa.
        let (permit, resp) = self
            .send(ticket, "renew", &host, Request::GetOnce(&url))
            .await?;
        let parsed =
            Self::ok_status(&resp).and_then(|()| (self.config.parsers.renewal)(&resp.body));
        self.finish(permit, &resp, parsed)
    }

    async fn logout(&self, ticket: &SessionTicket, session: &LoggedSession) -> Result<()> {
        let Some(path) = self.config.logout_path.as_deref() else {
            // Sem endpoint conhecido: os cookies morrem com o transporte.
            return Ok(());
        };
        let (url, host) = self.mobile_url(&session.base_url, path)?;
        let (permit, resp) = self
            .send(ticket, "logout", &host, Request::Get(&url))
            .await?;
        self.finish(permit, &resp, Self::ok_status(&resp))
    }
}
