# Guia de manutenção

Procedimentos para quem mantém o projeto. Arquitetura em [arquitetura.md](arquitetura.md).

> **Regra de ouro:** testes, CI e agentes **nunca** acessam o site real do Sophia. Use só `wiremock` + fixtures.

## 1. Atualizar seletores (quando o Sophia mudar o HTML)

Sintoma: ciclos terminando com `parse_error` ("HTML com estrutura inesperada") e o breaker abrindo.

1. Capture novas fixtures (seção 2).
2. Compare com as fixtures antigas em `crates/renovador-core/tests/fixtures/`.
3. Ajuste os seletores em `crates/renovador-core/src/sophia/selectors.rs` e, se preciso, `parser.rs`. Seletores atuais:
   - título: `div.ui-content > ul > li > div.div-textoLista > h3`;
   - link de renovação: `div.ui-content > ul > li > div.div-textoLista > div.ui-grid-a > div.ui-block-b > a`.
4. Estrutura desconhecida deve continuar retornando `ParseError`; nunca "chute".
5. Rode `cargo test --workspace` e publique uma nova versão.

## 2. Capturar e anonimizar fixtures

Feito manualmente, por uma pessoa, com a própria conta:

1. Abra `https://biblioteca.sophia.com.br/5362/mobile/`, vá em Renovações e faça login.
2. Salve a lista (Ctrl+S, "somente HTML") como `lista-emprestimos.html` (e `lista-vazia.html`, se possível).
3. Em DevTools › Rede, copie a resposta de `ajx_efetua_login.asp` com login certo e com senha errada: `login-ok.json` e `login-erro.json`.
4. Numa renovação real, salve `renovacao-sucesso.html`; em caso de recusa, `renovacao-recusa-<motivo>.html`.
5. **Anonimize** nome, login/matrícula, e-mail e códigos pessoais, mantendo o formato. Nunca commite dados pessoais.
6. Salve em `crates/renovador-core/tests/fixtures/`.

## 3. Política remota (kill switch)

O app baixa `policy.json` e `policy.json.sig` no máximo 1×/dia e só aceita a política se a assinatura Ed25519 conferir com a chave pública embutida. A política **só aperta**: cada limite efetivo é o mínimo entre o embutido e o remoto, `automation_enabled` vale `embutido E remoto` e `blocked_hosts` é a união das listas.

### 3.1 Gerar o par de chaves

Faça isso numa máquina confiável; a chave privada fica com a Editora, **fora do repositório**.

```sh
openssl genpkey -algorithm ed25519 -out policy-private.pem
openssl pkey -in policy-private.pem -pubout -out policy-public.pem
openssl pkey -pubin -in policy-public.pem -outform DER | tail -c 32 | base64
```

A última linha imprime a chave pública (32 bytes em base64) a embutir no núcleo (módulo `guard`). Guarde `policy-private.pem` em cofre de senhas/mídia offline, com cópia de segurança.

### 3.2 Assinar e publicar a política

1. Edite `policy.json` (por exemplo, `"automation_enabled": false` para desligar tudo, ou adicione hosts a `blocked_hosts`).
2. Assine os bytes exatos do arquivo:
   ```sh
   openssl pkeyutl -sign -rawin -inkey policy-private.pem -in policy.json | base64 > policy.json.sig
   ```
3. Publique os dois arquivos juntos, no endereço configurado no app.
4. Verifique antes de anunciar: `openssl pkeyutl -verify -rawin -pubin -inkey policy-public.pem -in policy.json -sigfile <(base64 -d policy.json.sig)`.

Não altere o arquivo depois de assinar: qualquer byte diferente invalida a assinatura e o app ignora a política remota (mantém os limites embutidos).

### 3.3 Rotacionar o par

1. Gere um novo par (3.1).
2. Publique uma versão do app com a nova chave pública embutida (ou com as duas, durante a transição, se o núcleo suportar).
3. Depois que as instalações atualizarem, passe a assinar só com a nova chave e destrua a antiga.
4. Se a chave privada vazar, rotacione imediatamente: uma chave vazada permite publicar uma política, mas como a política só aperta, o pior caso é desligar a automação.

## 4. Empacotamento

O workflow **Release** (`.github/workflows/release.yml`, só disparo manual) gera uma release **em rascunho** com:

| Plataforma | Arquivos | Observação |
|---|---|---|
| Windows | instalador NSIS (`-setup.exe`) e MSI | instala só para o usuário (sem admin); pt-BR |
| Linux | AppImage, `.deb` e `.rpm` | WebKitGTK 4.1 |
| macOS | `.dmg` universal (Intel + Apple Silicon) | sem assinatura Apple (ver 4.3) |
| Android | APK e AAB assinados + `SHA256SUMS-android.txt` | exige a keystore (4.2) |

Antes de compilar, o job `pré-checagens` confere que `THIRD-PARTY-NOTICES.md` e a lista da tela Sobre batem com os lockfiles e que a versão do MSI acompanha a do app.

### 4.1 Avisos de terceiros

`THIRD-PARTY-NOTICES.md` vai dentro de todos os instaladores. Sempre que mudar `Cargo.lock` ou `package-lock.json`:

```sh
cargo install cargo-about --locked --version 0.9.3 --features cli   # uma vez
node scripts/gen-notices.mjs
```

O script também regrava `src/lib/features/sobre/licenses.generated.json`. O CI (job "avisos de terceiros") falha se algum dos dois estiver desatualizado. A licença do GSAP não vem no pacote npm: a cópia oficial fica em [legal/licenca-gsap.txt](legal/licenca-gsap.txt). Se a Webflow mudar os termos (https://gsap.com/standard-license), atualize o arquivo e confira se o uso continua permitido.

### 4.2 Keystore do Android

Gerada uma única vez pela Editora, numa máquina confiável, e guardada **fora do repositório** (cofre de senhas + cópia offline). Perder a keystore impede atualizar o app já instalado nos aparelhos.

```sh
keytool -genkey -v -keystore renovador-release.jks -storetype PKCS12 -keyalg RSA -keysize 2048 -validity 10000 -alias renovador
base64 -w0 renovador-release.jks > renovador-release.jks.b64
```

Cadastre no environment `publicacao` (Settings › Environments) os secrets:

- `ANDROID_KEYSTORE_BASE64`: conteúdo de `renovador-release.jks.b64`;
- `ANDROID_KEY_ALIAS`: `renovador` (ou o alias escolhido);
- `ANDROID_KEY_PASSWORD`: a senha da keystore (a mesma da chave).

Apague o `.b64` depois. O workflow grava a keystore só no diretório temporário do runner e a apaga no fim. Sem os três secrets, o job Android falha com uma mensagem explicando o que falta.

Para gerar localmente um build assinado, crie `src-tauri/gen/android/keystore.properties` (ignorado pelo git) com `storeFile=<caminho absoluto>`, `keyAlias=...` e `password=...`. Sem esse arquivo, `tauri android build` sai sem assinatura.

### 4.3 Assinatura de código (desktop)

Os instaladores de Windows e macOS saem **sem assinatura**: o Windows mostra o aviso do SmartScreen e o macOS exige "Abrir mesmo assim" em Ajustes › Privacidade e Segurança. Para assinar é preciso um certificado de assinatura de código (Windows) e uma conta Apple Developer (macOS); ver https://v2.tauri.app/distribute/sign/windows/ e https://v2.tauri.app/distribute/sign/macos/.

### 4.4 Versão

A versão fica em `src-tauri/tauri.conf.json` (`version`) e em `package.json`. O MSI só aceita números (`maior.menor.patch.build`), então `bundle.windows.wix.version` precisa acompanhar: `2.0.0-alpha.0` → `2.0.0.0`; `2.0.1` → `2.0.1.0`. O versionCode do Android é derivado da versão pelo Tauri.

## 5. Checklist de release

Antes de disparar:

- [ ] Anuência por escrito da Biblioteca/SIBi UFTM arquivada ([análise preliminar](legal/analise-preliminar.md), [modelo de ofício](legal/modelo-oficio-anuencia.md)).
- [ ] Titularidade e licença confirmadas com a Editora/NIT; `publisher` e `copyright` em `tauri.conf.json` atualizados se mudar o titular.
- [ ] Par de chaves Ed25519 da política gerado (3.1), chave pública embutida no núcleo e `policy.json` assinado publicado.
- [ ] Versão atualizada em `tauri.conf.json`, `package.json` e `bundle.windows.wix.version` (4.4).
- [ ] CI verde em `main`: fmt, clippy, testes Rust, `npm run check`, Vitest, Android debug, cargo-deny, npm audit e avisos de terceiros.
- [ ] Testes de tela (Playwright + axe) passando localmente.
- [ ] Teste manual com conta real (seção 10 do plano): Windows e um Android físico — renovar 1 livro, conferir no site e no e-mail do Sophia; reiniciar no meio da sessão (cota persiste); cortar a rede (breaker abre); Android em Doze e modo avião.
- [ ] Manual do usuário revisado para a versão.

Disparar e revisar:

- [ ] Actions › Release › Run workflow; aprovar no environment `publicacao`.
- [ ] Na release em rascunho: baixar e instalar cada arquivo (Windows NSIS e MSI, um Linux, APK num aparelho); conferir a versão em Sobre e que `THIRD-PARTY-NOTICES.md` está instalado.
- [ ] Conferir os hashes de `SHA256SUMS-android.txt`.
- [ ] Escrever as notas da versão (o que mudou, problemas conhecidos) e só então publicar a release.
