# Aviso de privacidade

Este aviso explica como o Renovador de Livros trata seus dados, conforme a Lei Geral de Proteção de Dados (LGPD, Lei nº 13.709/2018).

## Resumo

- **Tudo fica no seu dispositivo.** O projeto não tem servidor e não recebe nenhum dado seu.
- **O banco de dados é criptografado** (SQLCipher, com chave aleatória de 256 bits guardada no cofre do sistema).
- **Guardar a senha é opcional.** No modo "sem salvar senha", o app pede a senha quando precisa entrar na biblioteca.
- Você pode exportar o histórico, fazer backup criptografado ou **apagar tudo** a qualquer momento.

## Quais dados são tratados

| Dado | Finalidade |
|---|---|
| Endereço da biblioteca, login e, se você quiser, a senha | Entrar no Terminal Web Sophia em seu nome |
| Títulos, datas de empréstimo e de devolução | Planejar renovações e lembretes |
| Histórico de tentativas de renovação | Mostrar resultados e estatísticas |
| Registro local de requisições (sem credenciais) | Aplicar os limites de uso e permitir diagnóstico |
| Preferências | Ajustes do app |

## Base legal

O tratamento é feito pelo próprio titular, no próprio dispositivo, para fins particulares e não econômicos (LGPD, art. 4º, I). O app apenas executa, a seu pedido, o que você faria manualmente no site da biblioteca.

## Proteção

- Banco de dados criptografado. A chave fica no cofre do sistema (Windows, macOS e Linux com Secret Service), no Android Keystore ou no Keychain do iOS. No Linux sem Secret Service, a chave é protegida por uma senha mestra (Argon2id).
- Segredos são apagados da memória após o uso.
- Backups são criptografados com uma senha escolhida por você.
- O registro de diagnóstico nunca contém senhas.

## Comunicação com terceiros

O app se comunica **apenas** com o endereço da biblioteca configurado (para a UFTM, o serviço Sophia) e, no máximo uma vez por dia, baixa a política remota de segurança (um arquivo público e assinado; nenhum dado seu é enviado). O tratamento de dados do lado da biblioteca segue as políticas da própria biblioteca e da Sophia.

## Seus direitos

Como os dados ficam só com você, você os controla diretamente: consulte-os no app, exporte o histórico (CSV/JSON), apague o histórico ou use **Ajustes › Apagar tudo**.

Veja também: [uso responsável](uso-responsavel.md).
