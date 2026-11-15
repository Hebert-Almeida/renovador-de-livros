# Modelo de ofício: pedido de anuência

> Modelo para adaptação. Preencha os campos entre colchetes antes de enviar. O contexto está na [análise preliminar](analise-preliminar.md).

---

**Ofício nº [número]/[ano] — Editora da UFTM**

[Cidade], [data].

À
**Coordenação do Sistema Integrado de Bibliotecas (SIBi) da Universidade Federal do Triângulo Mineiro**

**Assunto:** pedido de anuência para publicação do aplicativo "Renovador de Livros"

Prezados(as),

1. A Editora da UFTM apresenta o aplicativo de código aberto **Renovador de Livros**, que ajuda alunos e servidores a renovar seus empréstimos pelo Terminal Web Sophia da UFTM, usando as **próprias credenciais** e o mesmo serviço de autoatendimento já oferecido pela Biblioteca.

2. O aplicativo:
   1. funciona apenas no dispositivo do usuário; não há servidor do projeto e nenhum dado pessoal é enviado a terceiros;
   2. respeita integralmente as regras de renovação configuradas pela Biblioteca no Sophia (limite de renovações, reservas, bloqueios);
   3. identifica-se de forma honesta no cabeçalho `User-Agent`, sem se passar por navegador;
   4. aplica limites rígidos de uso (uma requisição por vez, intervalo mínimo de 2 segundos, no máximo 20 requisições por sessão e 2 sessões automáticas por conta ao dia), conforme o documento [uso responsável](uso-responsavel.md);
   5. **interrompe imediatamente** o uso diante de CAPTCHA, desafio de segurança ou bloqueio, sem tentar contorná-los;
   6. pode ter a automação desligada remotamente em todas as instalações, por meio de uma política assinada digitalmente, caso a Biblioteca solicite.

3. Solicitamos a **anuência por escrito** desta Coordenação para a publicação do aplicativo, bem como a confirmação das regras vigentes de renovação pela internet (quantidade de livros, prazo e número máximo de renovações).

4. Colocamo-nos à disposição para demonstração e para ajustar os limites conforme a orientação da Biblioteca. Caso a Biblioteca não concorde com a publicação, o aplicativo não será publicado.

Atenciosamente,

[Nome do responsável]
[Cargo] — Editora da UFTM
[Contato institucional]
