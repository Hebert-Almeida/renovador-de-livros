export default {
	title: 'Boas-vindas',
	intro: 'Configure o Renovador em poucos passos. Nada sai do seu computador além do acesso à biblioteca.',
	progressLabel: 'Etapas da configuração',
	stepOf: (n: number, total: number) => `Etapa ${n} de ${total}`,
	steps: {
		privacy: 'Privacidade',
		library: 'Biblioteca',
		account: 'Conta',
		test: 'Teste de login',
		notify: 'Notificações'
	},
	back: 'Voltar',
	next: 'Continuar',
	finish: 'Concluir',
	privacy: {
		heading: 'Privacidade e uso responsável',
		items: [
			'Sua matrícula e senha ficam só neste dispositivo, cifradas. Nenhum dado é enviado a terceiros.',
			'O app acessa a biblioteca com moderação: poucas requisições por dia, com intervalos e limites, para não sobrecarregar o sistema.',
			'As renovações seguem as regras da biblioteca. O app não burla limites, reservas nem verificações humanas.',
			'Você pode apagar todos os dados a qualquer momento em Ajustes.'
		],
		consent: 'Li e concordo com o aviso de privacidade e uso responsável',
		required: 'Para continuar, marque a caixa de consentimento.'
	},
	library: {
		heading: 'Escolha a biblioteca',
		legend: 'Biblioteca',
		supported: 'suportada',
		experimental: 'experimental',
		loading: 'Carregando bibliotecas…',
		advanced: 'Avançado: usar um endereço customizado',
		advancedHint: 'Para outra biblioteca que use o sistema Sophia.',
		urlLabel: 'Endereço da biblioteca Sophia',
		urlHint: 'Exemplo: https://biblioteca.sophia.com.br/5362',
		warning:
			'Atenção: só a biblioteca da UFTM é testada. Outros endereços são experimentais e podem não funcionar.',
		invalid: 'Endereço inválido.',
		required: 'Informe o endereço da biblioteca.',
		customName: 'Biblioteca customizada'
	},
	account: {
		heading: 'Sua conta na biblioteca',
		login: 'Matrícula ou login',
		password: 'Senha da biblioteca',
		passwordHint: 'Guardada cifrada neste dispositivo.',
		modeLegend: 'Modo de renovação',
		auto: 'Automático',
		autoHint: 'O app renova sozinho, alguns dias antes do vencimento.',
		manual: 'Manual',
		manualHint: 'O app só avisa; você decide quando renovar.',
		requiredLogin: 'Informe a matrícula ou login.',
		requiredPassword: 'Informe a senha.'
	},
	test: {
		heading: 'Teste de login',
		explain: 'Vamos entrar uma vez na biblioteca para conferir os dados. Esse teste conta na cota diária de acessos.',
		button: 'Testar login',
		ok: 'Login confirmado.',
		fail: 'A biblioteca recusou a matrícula ou a senha. Volte e corrija os dados.',
		skipNote: 'Você pode continuar sem testar; o login será verificado na primeira sincronização.'
	},
	notify: {
		heading: 'Notificações',
		explain: 'Avisamos quando um livro for renovado, quando algo falhar ou quando o prazo estiver perto.',
		allow: 'Permitir notificações',
		granted: 'Notificações permitidas.',
		denied: 'Notificações não permitidas. Você pode mudar isso em Ajustes.'
	},
	errors: {
		generic: 'Algo deu errado. Tente de novo.',
		network: 'Não foi possível conectar à biblioteca.',
		invalidCredentials: 'Matrícula ou senha incorretas.',
		human: 'A biblioteca pediu uma verificação humana. Entre pelo site e tente mais tarde.',
		quota: 'A cota de acessos foi atingida. Tente mais tarde.',
		guard: 'Acesso adiado para respeitar os limites da biblioteca. Tente mais tarde.',
		accountLimit: 'Limite de contas atingido.'
	}
} as const;
