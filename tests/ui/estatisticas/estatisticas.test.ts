import { describe, it, expect, afterEach, beforeAll, beforeEach, vi } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/svelte';
import { mockIPC, clearMocks } from '@tauri-apps/api/mocks';
import Page from '../../../src/routes/estatisticas/+page.svelte';
import { fillMonths, niceScale, rangeFor, formatPercent, monthRate } from '#lib/features/estatisticas/stats.ts';
import { overview, monthly } from './fixtures.ts';

beforeAll(() => {
	globalThis.ResizeObserver ??= class {
		observe() {}
		unobserve() {}
		disconnect() {}
	} as never;
	globalThis.matchMedia ??= ((q: string) => ({ matches: false, media: q, addEventListener() {}, removeEventListener() {} })) as never;
});
afterEach(() => {
	cleanup();
	clearMocks();
	vi.useRealTimers();
});

describe('stats.ts', () => {
	it('rangeFor começa no 1º dia do mês', () => {
		expect(rangeFor('6m', new Date(2026, 10, 15, 12))).toEqual({ from: '2026-06-01', to: '2026-11-15' });
		expect(rangeFor('12m', new Date(2026, 0, 3, 12))).toEqual({ from: '2025-02-01', to: '2026-01-03' });
	});
	it('rangeFor termina na data UTC de agora (o banco grava attempted_at em UTC)', () => {
		// 21h30 em São Paulo = 00h30 UTC do dia seguinte
		expect(rangeFor('6m', new Date('2026-11-16T00:30:00Z')).to).toBe('2026-11-16');
	});
	it('fillMonths cobre o período inteiro, inclusive na virada do ano', () => {
		const data = [
			{ month: '2026-01', renewals: 1, successes: 1, failures: 0 },
			{ month: '2025-11', renewals: 2, successes: 1, failures: 1 }
		];
		const out = fillMonths(data, '2026-03', '2025-10');
		expect(out.map((m) => m.month)).toEqual(['2025-10', '2025-11', '2025-12', '2026-01', '2026-02', '2026-03']);
		expect(out[2].renewals).toBe(0);
		// "desde o início": do primeiro mês com dados até o mês atual
		expect(fillMonths(data, '2026-02').map((m) => m.month)).toEqual(['2025-11', '2025-12', '2026-01', '2026-02']);
		expect(fillMonths([], '2026-02')).toEqual([]);
		expect(fillMonths([], '2026-02', '2026-01')).toHaveLength(2);
	});
	it('niceScale e taxas', () => {
		expect(niceScale(7)).toEqual({ top: 8, ticks: [0, 2, 4, 6, 8] });
		expect(niceScale(0).top).toBe(1);
		expect(monthRate({ month: 'x', renewals: 0, successes: 0, failures: 0 })).toBeNull();
		expect(formatPercent(0.5)).toMatch(/50\s?%/);
		expect(formatPercent(null)).toBeNull();
		expect(formatPercent(1)).toMatch(/100\s?%/);
		expect(formatPercent(0.995)).toMatch(/99,5\s?%/);
		expect(formatPercent(0.9999)).toMatch(/99,9\s?%/);
	});
});

describe('/estatisticas', () => {
	beforeEach(() => {
		vi.useFakeTimers({ toFake: ['Date'] });
		vi.setSystemTime(new Date(2026, 10, 16, 12));
	});

	it('mostra números, gráfico acessível e tabela', async () => {
		const calls: string[] = [];
		mockIPC((cmd) => {
			calls.push(cmd);
			if (cmd === 'stats_overview') return overview;
			if (cmd === 'stats_monthly') return monthly;
		});
		render(Page);
		expect(await screen.findByText('Livros distintos')).toBeInTheDocument();
		expect(screen.getByText('Dom Casmurro')).toBeInTheDocument();
		expect(screen.getByText(/75\s?%/)).toBeInTheDocument();
		// "Renovações feitas" conta só os sucessos (2 + 7), não as 12 tentativas
		expect(screen.getByText('Renovações feitas').nextElementSibling).toHaveTextContent(/^9$/);
		// 12m cobre dez/2025 a nov/2026, mesmo com dados só em set e nov
		expect(
			screen.getByRole('img', { name: /Gráfico de colunas com 12 meses, 12 tentativas de renovação no total; pico em/ })
		).toBeInTheDocument();
		expect(screen.getByRole('table')).toBeInTheDocument();
		expect(screen.getAllByRole('row')).toHaveLength(13);
		expect(calls).toEqual(['stats_overview', 'stats_monthly']);
	});

	it('troca de período recarrega', async () => {
		const ranges: unknown[] = [];
		mockIPC((cmd, args) => {
			if (cmd === 'stats_overview') {
				ranges.push((args as { range: unknown }).range);
				return overview;
			}
			return monthly;
		});
		render(Page);
		await screen.findByText('Livros distintos');
		await fireEvent.change(screen.getByLabelText('Período'), { target: { value: 'all' } });
		await screen.findByText('Livros distintos');
		expect(ranges).toHaveLength(2);
		expect((ranges[1] as { from: string }).from).toBe('2000-01-01');
	});

	it('estado vazio sem tentativas: taxa "—", não "0 %"', async () => {
		// o backend manda success_rate 0.0 quando não há tentativas
		mockIPC((cmd) =>
			cmd === 'stats_overview' ? { ...overview, renewals_total: 0, success_rate: 0, most_renewed: null } : []
		);
		render(Page);
		expect(await screen.findByText('Nenhuma tentativa de renovação neste período')).toBeInTheDocument();
		expect(screen.getByText('Nenhum livro renovado no período.')).toBeInTheDocument();
		expect(screen.getByText('Taxa de sucesso').parentElement).toHaveTextContent('—');
		expect(screen.queryByText(/0\s?%/)).toBeNull();
	});

	it('erro com tentar de novo', async () => {
		let fail = true;
		mockIPC((cmd) => {
			if (fail) throw new Error('falhou');
			return cmd === 'stats_overview' ? overview : monthly;
		});
		render(Page);
		const retry = await screen.findByRole('button', { name: 'Tentar de novo' });
		fail = false;
		await fireEvent.click(retry);
		expect(await screen.findByText('Livros distintos')).toBeInTheDocument();
		// o botão saiu do DOM; o foco vai para o conteúdo novo, não para o <body>
		await vi.waitFor(() => expect(document.activeElement).toBe(screen.getByRole('region', { name: 'Estatísticas' })));
	});
});
