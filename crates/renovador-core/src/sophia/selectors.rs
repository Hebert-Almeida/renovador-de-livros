//! Seletores e caminhos conhecidos do Terminal Web Sophia (mobile).
//! Ao atualizar, ver `docs/guia-de-manutencao.md` e as fixtures.

pub const LOGIN_PAGE: &str = "mobile/login.asp?idioma=ptbr&acesso=web&destino=6&detalhe=0";
pub const LOGIN_POST: &str = "mobile/ajx_efetua_login.asp";

/// Corpo do POST de login, na ordem do site. `codigo` e `senha` são preenchidos.
pub fn login_form<'a>(login: &'a str, password: &'a str) -> [(&'static str, &'a str); 10] {
    [
        ("idioma", "ptbr"),
        ("codigo", login),
        ("senha", password),
        ("usuarioexterno", "0"),
        ("detalhe", "0"),
        ("acesso", "web"),
        ("destino", "6"),
        ("midia", "0"),
        ("build_app", ""),
        ("mobile", "0"),
    ]
}

pub const LOAN_ITEM: &str = "div.ui-content > ul > li";
pub const LOAN_TITLE: &str = "div.ui-content > ul > li > div.div-textoLista > h3";
pub const LOAN_RENEW_LINK: &str =
    "div.ui-content > ul > li > div.div-textoLista > div.ui-grid-a > div.ui-block-b > a";

/// Marcadores de verificação humana: ao encontrar, abortar sem nova tentativa.
pub const HUMAN_VERIFICATION_MARKERS: &[&str] = &[
    "g-recaptcha",
    "h-captcha",
    "cf-turnstile",
    "captcha",
    "Just a moment",
    "cf-chl",
];
