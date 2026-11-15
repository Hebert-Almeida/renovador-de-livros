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

## 4. Checklist de release

- [ ] Anuência por escrito da Biblioteca/SIBi UFTM arquivada ([análise preliminar](legal/analise-preliminar.md)).
- [ ] `cargo test --workspace`, `npm run check`, `npm run test` e `npm run build` passam.
- [ ] `THIRD-PARTY-NOTICES.md` atualizado (incluindo a licença do GSAP).
- [ ] Release em rascunho aprovada no environment `publicacao`.
