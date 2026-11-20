//! Registro detalhado para diagnóstico: grava cada página recebida da
//! biblioteca numa pasta local, com um `registro.txt` que resume os passos.
//!
//! Desligado por padrão. As páginas trazem nome e empréstimos do usuário, por
//! isso ficam só no computador e são apagadas ao desligar. A senha nunca é
//! gravada: do POST de login, só os nomes dos campos aparecem no registro.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;

use super::{HttpResponse, Transport};
use crate::Result;

/// Arquivo cuja presença mantém o diagnóstico ligado entre execuções.
const MARKER: &str = ".ligado";
const LOG: &str = "registro.txt";
/// Página para acompanhar no navegador; recarrega sozinha.
pub const INDEX: &str = "index.html";
const INDEX_HEAD: &str = concat!(
    "<!doctype html><meta charset=\"utf-8\"><meta http-equiv=\"refresh\" content=\"5\">",
    "<title>Registro detalhado</title><style>body{font:14px system-ui;margin:16px}",
    "pre{white-space:pre-wrap;background:#f4f1ea;padding:8px;margin:0}section{margin:0 0 12px}</style>",
    "<h1>Registro detalhado</h1><p>Atualiza a cada 5 s. O passo mais novo fica no fim. ",
    "Clique no arquivo para ver a página recebida.</p>\n"
);
/// Páginas mantidas; as mais antigas são apagadas.
const MAX_PAGES: usize = 200;

/// Estado do diagnóstico, compartilhado entre o transporte e os comandos.
pub struct Tracer {
    dir: PathBuf,
    enabled: AtomicBool,
    seq: Mutex<u32>,
}

impl Tracer {
    /// `dir` é a pasta do diagnóstico; liga se o marcador já existir.
    pub fn new(dir: PathBuf) -> Self {
        let enabled = dir.join(MARKER).exists();
        let seq = next_seq(&dir);
        Self {
            dir,
            enabled: AtomicBool::new(enabled),
            seq: Mutex::new(seq),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Número de páginas gravadas.
    pub fn page_count(&self) -> u32 {
        pages(&self.dir).len() as u32
    }

    /// Liga (cria a pasta e o marcador) ou desliga (apaga a pasta inteira).
    pub fn set_enabled(&self, on: bool) -> std::io::Result<()> {
        if on {
            std::fs::create_dir_all(&self.dir)?;
            std::fs::write(self.dir.join(MARKER), b"")?;
        } else if self.dir.exists() {
            std::fs::remove_dir_all(&self.dir)?;
            *self.seq.lock().unwrap_or_else(|p| p.into_inner()) = 1;
        }
        self.enabled.store(on, Ordering::SeqCst);
        Ok(())
    }

    fn record(
        &self,
        method: &str,
        url: &str,
        form: Option<&[(&str, &str)]>,
        res: &Result<HttpResponse>,
    ) {
        if !self.enabled() {
            return;
        }
        if let Err(e) = self.try_record(method, url, form, res) {
            tracing::warn!("diagnóstico: não foi possível gravar: {e}");
        }
    }

    /// Caminho do `index.html`, criado se faltar.
    pub fn index_path(&self) -> std::io::Result<PathBuf> {
        std::fs::create_dir_all(&self.dir)?;
        let path = self.dir.join(INDEX);
        if !path.exists() {
            std::fs::write(&path, INDEX_HEAD)?;
        }
        Ok(path)
    }

    fn try_record(
        &self,
        method: &str,
        url: &str,
        form: Option<&[(&str, &str)]>,
        res: &Result<HttpResponse>,
    ) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        let n = {
            let mut seq = self.seq.lock().unwrap_or_else(|p| p.into_inner());
            let n = *seq;
            *seq += 1;
            n
        };
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let mut line = format!("[{n:04}] {now} {method} {url}\n");
        let mut file_link = None;
        if let Some(form) = form {
            let names: Vec<&str> = form.iter().map(|(k, _)| *k).collect();
            let _ = writeln!(
                line,
                "       campos enviados: {} (valores omitidos)",
                names.join(", ")
            );
        }
        match res {
            Ok(r) => {
                let file = format!("{n:04}-{}.{}", page_name(url), extension(r));
                std::fs::write(self.dir.join(&file), &r.body)?;
                file_link = Some(file.clone());
                let _ = writeln!(
                    line,
                    "       HTTP {} · {} bytes · arquivo {file}",
                    r.status,
                    r.body.len()
                );
                if r.final_url != url {
                    let _ = writeln!(line, "       redirecionado para {}", r.final_url);
                }
                let _ = writeln!(line, "       {}", summarize(&r.body));
            }
            Err(e) => {
                let _ = writeln!(line, "       ERRO: {e}");
            }
        }
        line.push('\n');
        let mut log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.dir.join(LOG))?;
        log.write_all(line.as_bytes())?;

        let mut row = format!("<section><pre>{}</pre>", escape(line.trim_end()));
        if let Some(f) = file_link {
            let _ = write!(
                row,
                "<a href=\"{0}\" target=\"_blank\">abrir {0}</a>",
                escape(&f)
            );
        }
        row.push_str("</section>\n");
        std::fs::OpenOptions::new()
            .append(true)
            .open(self.index_path()?)?
            .write_all(row.as_bytes())?;
        prune(&self.dir);
        Ok(())
    }
}

/// Transporte que repassa ao interno e registra no `Tracer`.
pub struct TracingTransport {
    inner: Arc<dyn Transport>,
    tracer: Arc<Tracer>,
}

impl TracingTransport {
    pub fn new(inner: Arc<dyn Transport>, tracer: Arc<Tracer>) -> Self {
        Self { inner, tracer }
    }
}

#[async_trait]
impl Transport for TracingTransport {
    async fn get(&self, url: &str) -> Result<HttpResponse> {
        let res = self.inner.get(url).await;
        self.tracer.record("GET", url, None, &res);
        res
    }

    async fn post_form(&self, url: &str, form: &[(&str, &str)]) -> Result<HttpResponse> {
        let res = self.inner.post_form(url, form).await;
        self.tracer.record("POST", url, Some(form), &res);
        res
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Nome curto da página: `renovacoes.asp?...` vira `renovacoes`.
fn page_name(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let last = path.rsplit('/').next().unwrap_or("");
    let stem = last.split('.').next().unwrap_or("");
    let clean: String = stem
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .take(40)
        .collect();
    if clean.is_empty() {
        "pagina".into()
    } else {
        clean
    }
}

fn extension(r: &HttpResponse) -> &'static str {
    let ct = r.header("content-type").unwrap_or("").to_ascii_lowercase();
    let body = r.body.trim_start();
    if ct.contains("json") || body.starts_with('{') {
        "json"
    } else if ct.contains("html") || body.starts_with('<') {
        "html"
    } else {
        "txt"
    }
}

/// Uma linha com o que ajuda a entender a página: título e o que foi achado.
fn summarize(body: &str) -> String {
    let lower = body.to_lowercase();
    if body.trim_start().starts_with('{') {
        let compact: String = body.split_whitespace().collect::<Vec<_>>().join(" ");
        let cut: String = compact.chars().take(300).collect();
        return format!("JSON: {cut}");
    }
    let title = lower
        .find("<title>")
        .and_then(|i| {
            let rest = &body[i + 7..];
            rest.find("</").map(|j| rest[..j].trim().to_owned())
        })
        .unwrap_or_default();
    let mut parts = vec![format!("título \"{title}\"")];
    parts.push(format!(
        "{} item(ns) de empréstimo (div-textoLista)",
        lower.matches("div-textolista").count()
    ));
    parts.push(format!(
        "{} link(s) de renovação (renovar.asp)",
        lower.matches("renovar.asp").count()
    ));
    if lower.contains("id=\"senha\"") || lower.contains("id=\"login\"") {
        parts.push("tem formulário de login".into());
    }
    if super::parser::needs_human_verification(body) {
        parts.push("tem verificação humana (captcha)".into());
    }
    parts.join(" · ")
}

fn pages(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.len() > 5 && n.as_bytes()[..4].iter().all(u8::is_ascii_digit))
        })
        .collect();
    v.sort();
    v
}

fn next_seq(dir: &Path) -> u32 {
    pages(dir)
        .last()
        .and_then(|p| p.file_name()?.to_str()?.get(..4)?.parse::<u32>().ok())
        .map_or(1, |n| n + 1)
}

fn prune(dir: &Path) {
    let all = pages(dir);
    if all.len() > MAX_PAGES {
        for p in &all[..all.len() - MAX_PAGES] {
            let _ = std::fs::remove_file(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Echo;

    #[async_trait]
    impl Transport for Echo {
        async fn get(&self, url: &str) -> Result<HttpResponse> {
            Ok(HttpResponse {
                status: 200,
                final_url: url.into(),
                headers: vec![("content-type".into(), "text/html".into())],
                body: "<html><title>Lista</title><div class=\"div-textoLista\"></div></html>"
                    .into(),
            })
        }
        async fn post_form(&self, url: &str, _form: &[(&str, &str)]) -> Result<HttpResponse> {
            Ok(HttpResponse {
                status: 200,
                final_url: url.into(),
                headers: vec![],
                body: r#"{"login_valido":"true"}"#.into(),
            })
        }
    }

    fn tmp() -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "renovador-trace-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[tokio::test]
    async fn off_by_default_writes_nothing() {
        let dir = tmp();
        let tracer = Arc::new(Tracer::new(dir.clone()));
        let t = TracingTransport::new(Arc::new(Echo), tracer.clone());
        t.get("https://x.test/5362/mobile/renovacoes.asp?a=1")
            .await
            .unwrap();
        assert!(!tracer.enabled());
        assert!(!dir.exists());
    }

    #[tokio::test]
    async fn records_pages_without_password_and_wipes_on_disable() {
        let dir = tmp();
        let tracer = Arc::new(Tracer::new(dir.clone()));
        tracer.set_enabled(true).unwrap();
        let t = TracingTransport::new(Arc::new(Echo), tracer.clone());
        t.post_form(
            "https://x.test/m/ajx_efetua_login.asp",
            &[("codigo", "aluno"), ("senha", "segredo")],
        )
        .await
        .unwrap();
        t.get("https://x.test/5362/mobile/renovacoes.asp?a=1")
            .await
            .unwrap();
        assert_eq!(tracer.page_count(), 2);
        assert!(dir.join("0001-ajx_efetua_login.json").exists());
        assert!(dir.join("0002-renovacoes.html").exists());
        let log = std::fs::read_to_string(dir.join(LOG)).unwrap();
        assert!(log.contains("campos enviados: codigo, senha"));
        assert!(!log.contains("segredo") && !log.contains("aluno"));
        assert!(log.contains("1 item(ns) de empréstimo"));
        let index = std::fs::read_to_string(dir.join(INDEX)).unwrap();
        assert!(index.contains("href=\"0002-renovacoes.html\""));
        assert!(!index.contains("segredo"));

        // Ao reabrir, continua ligado e numera a partir da última página.
        let again = Tracer::new(dir.clone());
        assert!(again.enabled());
        assert_eq!(next_seq(&dir), 3);

        tracer.set_enabled(false).unwrap();
        assert!(!dir.exists());
    }
}
