//! Detecção de respostas que exigem abortar o ciclo sem nova tentativa
//! (seção 5.1): CAPTCHA, desafio do Cloudflare e host que não é Sophia.

use url::Url;

use super::HttpResponse;
use super::parser::needs_human_verification;

/// Problema detectado numa resposta do Sophia.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Detection {
    /// CAPTCHA ou desafio do Cloudflare.
    HumanVerification,
    /// A resposta terminou em outro host (redirecionamento para fora do Sophia).
    ForeignHost { host: String },
    /// 429 ou 503: o servidor pediu para aguardar.
    RateLimited {
        retry_after: Option<std::time::Duration>,
    },
    /// Outros 5xx.
    ServerError,
}

/// `true` se o cabeçalho `x-powered-by` identifica o Sophia.
pub fn is_sophia_powered(resp: &HttpResponse) -> bool {
    resp.header("x-powered-by")
        .is_some_and(|v| v.to_ascii_lowercase().contains("sophia"))
}

/// Marcas do Terminal Web Sophia (mobile) no HTML: título padrão, logotipo
/// e o endpoint de login usado pelo formulário.
const SOPHIA_PAGE_MARKERS: &[&str] = &["sophia biblioteca", "logotiposophia", "ajx_efetua_login"];

/// `true` se a resposta parece vir de um Terminal Web Sophia: pelo
/// cabeçalho `x-powered-by` ou, como o servidor nem sempre o envia, por
/// marcas da própria página.
pub fn is_sophia_site(resp: &HttpResponse) -> bool {
    if is_sophia_powered(resp) {
        return true;
    }
    let body = resp.body.to_ascii_lowercase();
    SOPHIA_PAGE_MARKERS.iter().any(|m| body.contains(m))
}

/// Host (com porta, se houver) de uma URL.
pub fn host_of(url: &str) -> Option<String> {
    let u = Url::parse(url).ok()?;
    let host = u.host_str()?.to_ascii_lowercase();
    Some(match u.port() {
        Some(p) => format!("{host}:{p}"),
        None => host,
    })
}

/// Lê `Retry-After` em segundos (o formato de data HTTP é ignorado).
pub fn retry_after(resp: &HttpResponse) -> Option<std::time::Duration> {
    resp.header("retry-after")?
        .trim()
        .parse::<u64>()
        .ok()
        .map(std::time::Duration::from_secs)
}

/// Inspeciona a resposta. A verificação humana tem prioridade, pois o
/// Cloudflare costuma responder o desafio com 403/503.
pub fn inspect(expected_host: &str, resp: &HttpResponse) -> Option<Detection> {
    if needs_human_verification(&resp.body) || resp.header("cf-mitigated").is_some() {
        return Some(Detection::HumanVerification);
    }
    match host_of(&resp.final_url) {
        Some(h) if h.eq_ignore_ascii_case(expected_host) => {}
        Some(host) => return Some(Detection::ForeignHost { host }),
        None => {
            return Some(Detection::ForeignHost {
                host: resp.final_url.clone(),
            });
        }
    }
    match resp.status {
        429 | 503 => Some(Detection::RateLimited {
            retry_after: retry_after(resp),
        }),
        500..=599 => Some(Detection::ServerError),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resp(status: u16, url: &str, body: &str) -> HttpResponse {
        HttpResponse {
            status,
            final_url: url.into(),
            headers: vec![("Retry-After".into(), "120".into())],
            body: body.into(),
        }
    }

    #[test]
    fn detects_each_case() {
        let h = "biblioteca.sophia.com.br";
        let ok = "https://biblioteca.sophia.com.br/5362/mobile/x.asp";
        assert_eq!(inspect(h, &resp(200, ok, "<h3>Livro</h3>")), None);
        assert_eq!(
            inspect(h, &resp(503, ok, "<div class=\"cf-turnstile\">")),
            Some(Detection::HumanVerification)
        );
        assert_eq!(
            inspect(h, &resp(200, "https://evil.example/x", "")),
            Some(Detection::ForeignHost {
                host: "evil.example".into()
            })
        );
        assert_eq!(
            inspect(h, &resp(429, ok, "")),
            Some(Detection::RateLimited {
                retry_after: Some(std::time::Duration::from_secs(120))
            })
        );
        assert_eq!(inspect(h, &resp(502, ok, "")), Some(Detection::ServerError));
    }
}
