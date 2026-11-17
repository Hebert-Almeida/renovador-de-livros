const int = new Intl.NumberFormat('pt-BR');
const n = (v: number, um: string, varios: string) => `${int.format(v)} ${v === 1 ? um : varios}`;

export default {
	title: 'Estatísticas',
	intro: 'Um resumo do que o Renovador fez por você no período escolhido.',
	period: 'Período',
	periods: {
		'6m': 'Últimos 6 meses',
		'12m': 'Últimos 12 meses',
		all: 'Desde o início'
	},
	loading: 'Carregando estatísticas…',
	errorTitle: 'Não foi possível carregar as estatísticas',
	retry: 'Tentar de novo',
	emptyTitle: 'Nenhuma tentativa de renovação neste período',
	emptyDescription: 'Escolha um período maior ou volte depois que o Renovador tentar renovar algum empréstimo.',
	tiles: {
		distinctBooks: 'Livros distintos',
		activeLoans: 'Empréstimos ativos',
		renewals: 'Renovações feitas',
		successRate: 'Taxa de sucesso',
		daysGained: 'Dias ganhos'
	},
	noRate: '—',
	mostRenewed: 'Livro mais renovado',
	noMostRenewed: 'Nenhum livro renovado no período.',
	monthlyTitle: 'Tentativas de renovação por mês',
	chartSummary: (months: number, total: number, peak: string | null) =>
		`Gráfico de colunas com ${n(months, 'mês', 'meses')}, ${n(total, 'tentativa de renovação', 'tentativas de renovação')} no total` +
		(peak ? `; pico em ${peak}` : '') +
		'. Os mesmos dados estão na tabela abaixo.',
	tableCaption: 'Tentativas de renovação por mês (tabela)',
	cols: { month: 'Mês', renewals: 'Tentativas', successes: 'Sucessos', failures: 'Falhas', rate: 'Taxa de sucesso' },
	tooltip: (tries: number, ok: number, fail: number) =>
		`${n(tries, 'tentativa', 'tentativas')} · ${int.format(ok)} com sucesso · ${int.format(fail)} com falha`
} as const;
