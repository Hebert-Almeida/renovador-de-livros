# Renovador de Livros

Automatizador de renovação de empréstimos para o Terminal Web **Sophia** da Biblioteca da Universidade Federal do Triângulo Mineiro (UFTM), desenvolvido para a Editora da UFTM.

> **Status:** versão 2.0 em desenvolvimento. **A publicação está condicionada à anuência por escrito da Biblioteca/SIBi UFTM** (ver a [análise preliminar](docs/legal/analise-preliminar.md)).

## Funcionalidades (2.0)

- Renovação e reagendamento automáticos, **confirmando a nova data** após cada renovação.
- Lembretes e notificações (renovado, falhou, vencendo, senha inválida, automação pausada).
- Agenda de renovações, histórico e estatísticas.
- Windows, Linux e Android; macOS e iOS desejáveis.
- Interface acessível (WCAG 2.2 AA + eMAG).
- Limites rígidos de uso para não sobrecarregar o serviço da biblioteca ([uso responsável](docs/legal/uso-responsavel.md)).

## Privacidade e credenciais

O app **pode guardar sua senha**, se você quiser, para renovar em segundo plano. Nesse caso ela fica num **banco local criptografado** (SQLCipher), cuja chave é protegida pelo cofre do sistema. Também há o modo "sem salvar senha". Nada é enviado a servidores do projeto: o app só conversa com a biblioteca. Detalhes no [aviso de privacidade](docs/legal/aviso-de-privacidade.md).

## Tecnologias

- [Tauri 2](https://tauri.app/) com núcleo em **Rust** (`crates/renovador-core`)
- **SvelteKit** (SPA) + TypeScript
- SQLCipher e reqwest (HTTP puro, sem navegador embutido)

## Desenvolvimento

```sh
npm install
npm run tauri dev       # abre o app
cargo test --workspace  # testes do núcleo
npm run check && npm run test
```

Documentação:

- [Arquitetura](docs/arquitetura.md)
- [Guia de manutenção](docs/guia-de-manutencao.md)
- [Manual do usuário](docs/manual-do-usuario.md) (em construção)
- [Plano da versão 2.0](docs/PLANO-V2.md)

## Licença

Licenciado, à sua escolha, sob [MIT](LICENSE-MIT) **ou** [Apache-2.0](LICENSE-APACHE) (`MIT OR Apache-2.0`). As versões anteriores (1.x, em Electron) continuam sob GPL-3.0.

## Aviso

Este projeto **não tem vínculo** com a Sophia nem com a Prima. "Sophia" é marca de terceiros, citada apenas para identificar o sistema com que o app é compatível. Bibliotecas Sophia além da UFTM são experimentais e ficam sob responsabilidade do usuário.
