export default {
	title: 'Ajustes',
	intro: 'Preferências deste dispositivo. As mudanças são salvas na hora.',
	loading: 'Carregando ajustes…',
	loadError: 'Não foi possível carregar os ajustes.',
	retry: 'Tentar de novo',
	saved: 'Ajuste salvo.',
	saveError: 'Não foi possível salvar o ajuste.',
	schedule: {
		heading: 'Antecedência, horário e notificações',
		rules: 'Regras de agendamento',
		renewBefore: 'Renovação automática com a antecedência de cada conta (padrão: 1 dia antes), ajustável em Contas.',
		retry: 'Nova tentativa no dia do vencimento só se a falha foi de rede.',
		reminders: 'Lembretes com a antecedência de cada conta (padrão: 2 dias antes) e no dia do vencimento.',
		window: 'Pedidos à biblioteca apenas entre 07h e 21h (horário de Brasília).',
		sync: 'Sincronização de novos empréstimos a cada 3 dias.',
		period: 'Período da renovação automática',
		periodHint:
			'O minuto exato é sorteado dentro do período para não sobrecarregar a biblioteca. Vale para todas as contas.',
		periods: {
			auto: 'Qualquer hora (07h–21h)',
			morning: 'Manhã (07h–12h)',
			afternoon: 'Tarde (12h–17h)',
			evening: 'Noite (17h–21h)'
		},
		notifications: 'Notificações',
		notificationsHint: 'Avisa quando um livro é renovado, falha, está vencendo ou a senha ficou inválida.'
	},
	appearance: {
		heading: 'Aparência e movimento',
		theme: 'Tema',
		themes: { system: 'Seguir o sistema', light: 'Claro', dark: 'Escuro', contrast: 'Alto contraste' },
		motion: 'Movimento',
		motions: { system: 'Seguir o sistema', reduced: 'Reduzido', full: 'Completo' }
	},
	system: {
		heading: 'Sistema e segurança',
		autostart: 'Iniciar com o sistema',
		autostartHint: 'Mantém as renovações em dia sem precisar abrir o app.',
		appLock: 'Bloquear o app com a senha mestra',
		appLockHint: 'Pede a senha mestra sempre que o app for aberto.'
	},
	data: {
		heading: 'Backup e dados',
		path: 'Arquivo de backup',
		pathHint: 'Caminho completo do arquivo, por exemplo C:\Backups\renovador.bak',
		pick: 'Escolher arquivo…',
		passphrase: 'Senha do backup',
		passphraseHint: 'Mínimo de 8 caracteres. Sem ela o backup não pode ser restaurado.',
		passphraseShort: 'A senha do backup precisa ter pelo menos 8 caracteres.',
		pathMissing: 'Informe o arquivo de backup.',
		create: 'Criar backup',
		created: 'Backup criado.',
		restore: 'Restaurar backup…',
		restoreTitle: 'Restaurar backup?',
		restoreBody: 'Os dados atuais deste dispositivo serão substituídos pelos do backup.',
		restoreConfirm: 'Restaurar',
		restored: 'Backup restaurado.',
		cancel: 'Cancelar',
		failed: 'A operação falhou',
		wipe: 'Apagar tudo…',
		wipeTitle: 'Apagar todos os dados?',
		wipeBody: 'Contas, senhas, empréstimos, agenda e histórico serão apagados deste dispositivo. Não há como desfazer.',
		wipeWord: 'APAGAR',
		wipeLabel: 'Digite APAGAR para confirmar',
		wipeConfirm: 'Apagar definitivamente',
		wiped: 'Todos os dados foram apagados.'
	}
} as const;
