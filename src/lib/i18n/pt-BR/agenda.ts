export default {
	title: 'Agenda',
	intro: 'Renovações, sincronizações e lembretes planejados. Horários de Brasília; execução automática só entre 07:00 e 21:00.',
	filterLabel: 'Mostrar',
	filters: {
		all: 'Todos',
		pending: 'Agendados',
		paused: 'Pausados',
		cancelled: 'Cancelados',
		failed: 'Com falha',
		done: 'Concluídos'
	},
	listLabel: 'Agendamentos',
	loading: 'Carregando agendamentos…',
	loadError: 'Não foi possível carregar a agenda.',
	retry: 'Tentar de novo',
	emptyTitle: 'Nada agendado',
	emptyDescription: 'Quando houver renovações ou lembretes planejados, eles aparecem aqui.',
	kind: { renew: 'Renovação', sync: 'Sincronização', remind: 'Lembrete' },
	status: {
		pending: 'Agendado',
		paused: 'Pausado',
		cancelled: 'Cancelado',
		failed: 'Falhou',
		done: 'Concluído'
	},
	origin: { system: 'criado automaticamente', user: 'criado por você' },
	account: (id: number) => `Conta ${id}`,
	loan: (id: number) => `empréstimo nº ${id}`,
	runAt: 'Execução em',
	actions: {
		pause: 'Pausar',
		reactivate: 'Reativar',
		reschedule: 'Reagendar',
		cancel: 'Cancelar',
		cancelRenew: 'Cancelar renovação agendada'
	},
	actionLabel: (action: string, what: string, when: string) => `${action}: ${what} de ${when}`,
	done: {
		pause: 'Agendamento pausado.',
		reactivate: 'Agendamento reativado.',
		reschedule: 'Agendamento reagendado.',
		cancel: 'Agendamento cancelado.',
		cancelRenew: 'Renovação agendada cancelada.'
	},
	actionError: 'Não foi possível concluir a ação.',
	cancelTitle: 'Cancelar agendamento?',
	cancelRenewTitle: 'Cancelar renovação agendada?',
	cancelBody: (what: string, when: string) =>
		`${what} de ${when} não será executado. Agendamentos cancelados por você não são recriados automaticamente; você pode reativá-lo depois.`,
	keep: 'Manter',
	confirmCancel: 'Sim, cancelar',
	rescheduleTitle: 'Reagendar',
	rescheduleField: 'Nova data e hora (horário de Brasília)',
	rescheduleHint: 'Permitido entre 07:00 e 20:59.',
	rescheduleErrors: {
		invalid: 'Informe uma data e hora válidas.',
		past: 'Escolha um horário no futuro.',
		window: 'Fora da janela permitida: escolha entre 07:00 e 20:59.'
	},
	save: 'Salvar',
	close: 'Fechar'
} as const;
