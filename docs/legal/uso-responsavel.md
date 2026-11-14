# Uso responsável

O Renovador de Livros usa o serviço de autoatendimento da biblioteca com moderação. Os limites abaixo são **fixos no código** ([`guard/limits.rs`](../../crates/renovador-core/src/guard/limits.rs)) e não podem ser alterados por arquivo de configuração, linha de comando ou variável de ambiente. A política remota pode apenas torná-los mais rígidos.

## Limites

| Regra | Valor |
|---|---|
| Requisições simultâneas | 1 (global) |
| Intervalo entre requisições | ≥ 2 s + variação aleatória de 0–1 s |
| Máximo de requisições por sessão | 20 |
| Sessões automáticas por conta/dia | ≤ 2 |
| Sessões manuais por conta/dia | ≤ 4, com espera ≥ 30 min entre elas |
| Sessões por instalação/dia | ≤ 10 |
| Contas por instalação | ≤ 3 (padrão: 1) |
| Logins distintos testados por instalação/dia | ≤ 3; senha errada pausa a conta até você editar a senha |
| Falhas consecutivas (rede, erro 5xx, página inesperada) | 3 → pausa de 6 h; 5 → pausa de 24 h + notificação |
| Resposta HTTP 429/503 | respeita `Retry-After`, mínimo de 1 h |
| CAPTCHA, desafio do Cloudflare ou bloqueio | para na hora, sem nova tentativa; pede ação manual e abre o site oficial |
| Endereço da biblioteca | só HTTPS e precisa se identificar como Sophia; cotas aplicadas também por servidor |
| Horário automático | 07h–21h, com minuto aleatório fixo por instalação |
| Política remota (kill switch) | consultada no máximo 1×/dia; só aperta, nunca afrouxa |

As cotas ficam gravadas no banco local: reiniciar o app não as zera. Só uma instância do app roda por vez.

## Compromissos

- O app se identifica honestamente (`User-Agent: RenovadorLivrosUFTM/<versão>`), não se passa por navegador e não baixa imagens, CSS ou scripts.
- Nunca tenta contornar CAPTCHA, bloqueios ou regras da biblioteca (reservas, limite de renovações, atrasos).
- Use apenas a **sua** conta.
- Bibliotecas Sophia além da UFTM são **experimentais** e ficam sob sua responsabilidade.

Veja também: [aviso de privacidade](aviso-de-privacidade.md) e [análise preliminar](analise-preliminar.md).
