export default {
	title: 'Sobre',
	intro: 'Renovação automática e responsável de empréstimos na Biblioteca Sophia (UFTM).',
	version: {
		heading: 'Versão',
		label: 'Versão instalada',
		loading: 'Carregando versão…',
		unavailable: 'Indisponível',
		license: 'Licença do projeto: MIT OR Apache-2.0.',
		disclaimer: 'Projeto independente, sem vínculo com a Sophia/Prima. "Sophia" é marca de terceiros.'
	},
	legal: {
		heading: 'Privacidade e uso responsável',
		tabsLabel: 'Textos legais',
		privacyTab: 'Aviso de privacidade',
		usageTab: 'Uso responsável',
		privacy: {
			intro: 'Como o Renovador de Livros trata seus dados, conforme a LGPD (Lei nº 13.709/2018).',
			points: [
				'Tudo fica no seu dispositivo. O projeto não tem servidor e não recebe nenhum dado seu.',
				'O banco de dados é criptografado (SQLCipher, com chave aleatória de 256 bits guardada no cofre do sistema).',
				'Guardar a senha é opcional. No modo "sem salvar senha", o app pede a senha quando precisa entrar na biblioteca.',
				'O app se comunica apenas com o endereço da biblioteca configurado e, no máximo uma vez por dia, baixa a política remota de segurança (nenhum dado seu é enviado).',
				'O registro de diagnóstico nunca contém senhas.',
				'Você pode exportar o histórico, fazer backup criptografado ou apagar tudo a qualquer momento (Ajustes › Apagar tudo).'
			],
			source: 'Texto completo: docs/legal/aviso-de-privacidade.md'
		},
		usage: {
			intro: 'Os limites são fixos no código e não podem ser alterados por configuração. A política remota pode apenas torná-los mais rígidos.',
			points: [
				'Uma requisição por vez, com intervalo de pelo menos 2 s mais variação aleatória.',
				'No máximo 2 sessões automáticas e 4 manuais por conta por dia; até 10 sessões por instalação por dia.',
				'Falhas seguidas pausam a conta (3 falhas: 6 h; 5 falhas: 24 h).',
				'CAPTCHA, desafio ou bloqueio: o app para na hora e pede ação manual no site oficial.',
				'Renovações automáticas só entre 07h e 21h.',
				'O app se identifica honestamente e nunca tenta contornar regras da biblioteca.',
				'Use apenas a sua conta. Bibliotecas além da UFTM são experimentais.'
			],
			source: 'Texto completo: docs/legal/uso-responsavel.md'
		}
	},
	licenses: {
		heading: 'Licenças de terceiros',
		caption: 'Bibliotecas JavaScript usadas pela interface',
		name: 'Pacote',
		versionCol: 'Versão',
		license: 'Licença',
		rustSummary: (n: number) => `Bibliotecas Rust do núcleo (${n})`,
		rustCaption: 'Bibliotecas Rust compiladas no app (desktop e Android)',
		note: 'Listas geradas a partir de package-lock.json e Cargo.lock; as licenças Rust também são verificadas pelo cargo-deny. Os textos completos estão em THIRD-PARTY-NOTICES.md, instalado junto com o app.'
	},
	diagnostics: {
		heading: 'Diagnóstico',
		description: 'Registro local das últimas requisições à biblioteca. Não contém senhas nem outras credenciais.',
		caption: 'Últimas requisições',
		refresh: 'Atualizar registro',
		empty: 'Nenhuma requisição registrada.',
		error: 'Não foi possível carregar o registro.',
		loaded: (n: number) => (n === 1 ? '1 requisição carregada.' : `${n} requisições carregadas.`),
		at: 'Data e hora',
		host: 'Servidor',
		kind: 'Tipo',
		status: 'HTTP',
		outcome: 'Resultado',
		account: 'Conta'
	},
	trace: {
		heading: 'Registro detalhado',
		description:
			'Grava no computador cada página que o app recebe da biblioteca, com um arquivo registro.txt que resume cada passo. Serve para entender por que algo não funcionou.',
		toggle: 'Gravar as páginas recebidas',
		toggleHint: 'As páginas têm seu nome e seus empréstimos. Ficam só neste computador e são apagadas ao desligar. A senha nunca é gravada.',
		open: 'Abrir pasta',
		folder: 'Pasta',
		pages: (n: number) => (n === 1 ? '1 página gravada.' : `${n} páginas gravadas.`),
		howTo: 'Ligue, sincronize a conta e depois abra a pasta. Comece pelo registro.txt.',
		unavailable: 'O registro detalhado não está disponível.',
		error: 'Não foi possível alterar o registro detalhado.',
		openError: 'Não foi possível abrir a pasta.',
		on: 'Registro detalhado ligado.',
		off: 'Registro detalhado desligado e páginas apagadas.'
	}
} as const;
