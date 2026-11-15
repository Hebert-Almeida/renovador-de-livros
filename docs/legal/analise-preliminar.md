# Análise jurídica preliminar

> **Aviso:** este documento resume uma pesquisa feita pela equipe do projeto. **Não é parecer jurídico** e não substitui a análise da Procuradoria, do NIT ou de assessoria jurídica da UFTM.

## Conclusão

Nenhum impedimento legal foi encontrado ao uso de um automatizador local, operado pelo próprio titular da conta, para renovar empréstimos no Terminal Web Sophia. Mesmo assim, **a publicação depende de anuência por escrito da Biblioteca/SIBi UFTM** (ver o [modelo de ofício](modelo-oficio-anuencia.md)).

## Fundamentos

- O Sophia oferece renovação por **autoatendimento** no Terminal Web (Descritivo Funcional, item 20.25). A biblioteca configura os limites e o servidor os aplica (itens 13.12.3.1 e 5.1.15). O app não contorna nenhum desses limites.
- A UFTM permite renovar pela internet (5 livros, 7 dias, até 15 renovações sem reserva — *a confirmar com a Biblioteca*).
- O `robots.txt` do serviço libera `/5362/mobile/` para agentes genéricos; bloqueia apenas `/gerenciador/`, `/terminal/`, `/terminalri/` e crawlers de IA.
- Não foram encontrados termos de uso públicos que proíbam automação.
- A API Web de Integração oficial não cobre renovação.
- Legislação:
  - **Código Penal, art. 154-A**: exige acesso *sem autorização* ou violação de mecanismo de segurança. O usuário usa a própria credencial e o app nunca burla segurança; não se enquadra.
  - **Código Penal, art. 266, §1º**: fundamenta os limites anti-abuso (anti-DDoS) do app, descritos em [uso responsável](uso-responsavel.md).
  - **LGPD, art. 4º, I**: o tratamento é local e feito pelo próprio titular, para fins particulares. Ver o [aviso de privacidade](aviso-de-privacidade.md).

## Obrigações assumidas pelo projeto

1. Obter anuência por escrito da Biblioteca/SIBi UFTM antes de publicar (também atende à diretriz 5.2.2 da App Store).
2. Nunca contornar CAPTCHA ou bloqueio. O Sophia permite que a biblioteca ative CAPTCHA (itens 20.2.3.1 e 20.37); se isso ocorrer, o app para e pede ação manual.
3. Confirmar a titularidade e o relicenciamento (`MIT OR Apache-2.0`) com a Editora/NIT. As versões antigas continuam sob GPL-3.0.
4. Tratar outras bibliotecas Sophia como **experimentais**, sob responsabilidade do usuário.

## Critério de parada

Se a Biblioteca UFTM ou a Sophia negarem a anuência ou pedirem que o uso cesse, o projeto para e a política remota (kill switch) desliga a automação em todas as instalações. O procedimento está no [guia de manutenção](../guia-de-manutencao.md#3-política-remota-kill-switch).

## Fontes

- robots.txt: <https://biblioteca.sophia.com.br/robots.txt>
- Descritivo Funcional do Sophia Biblioteca Web: <https://www.ifb.edu.br/attachments/article/39100/Descritivo%20Funcional%20do%20Sophia%20Biblioteca%20Web.pdf>
- API Web de Integração: <https://ifb.edu.br/attachments/article/39100/API%20Web%20de%20Integra%C3%A7%C3%A3o.pdf>
- Regras de empréstimo da UFTM: <https://sistemas.uftm.edu.br/integrado/sistemas/pub/publicacao.html?secao=1527&publicacao=10973>
- Política de privacidade da Sophia: <https://sophia.com.br/politica-de-privacidade/>
- Licença do GSAP: <https://gsap.com/community/standard-license/>
- WorkManager no Tauri (issue #14530): <https://github.com/tauri-apps/tauri/issues/14530>
