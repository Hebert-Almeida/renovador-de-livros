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
	emptyTitle: 'Ainda não há renovações',
	emptyDescription: 'Assim que o Renovador renovar algum empréstimo, os números aparecem aqui.',
	tiles: {
		distinctBooks: 'Livros distintos',
		activeLoans: 'Empréstimos ativos',
		renewals: 'Renovações',
		successRate: 'Taxa de sucesso',
		daysGained: 'Dias ganhos'
	},
	noRate: '—',
	mostRenewed: 'Livro mais renovado',
	noMostRenewed: 'Nenhum livro renovado no período.',
	monthlyTitle: 'Renovações por mês',
	chartSummary: (n: number, total: number, peak: string) =>
		`Gráfico de colunas com ${n} ${n === 1 ? 'mês' : 'meses'}, ${total} renovações no total; pico em ${peak}. Os mesmos dados estão na tabela abaixo.`,
	tableCaption: 'Renovações por mês (tabela)',
	cols: { month: 'Mês', renewals: 'Renovações', successes: 'Sucessos', failures: 'Falhas', rate: 'Taxa de sucesso' },
	tooltip: (renewals: number, ok: number, fail: number) =>
		`${renewals} renovações · ${ok} com sucesso · ${fail} com falha`
} as const;
