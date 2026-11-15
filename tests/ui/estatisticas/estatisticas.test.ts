import { describe, it, expect, afterEach, beforeAll } from 'vitest';
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
});

describe('stats.ts', () => {
	it('rangeFor começa no 1º dia do mês', () => {
		expect(rangeFor('6m', new Date(2026, 10, 15))).toEqual({ from: '2026-06-01', to: '2026-11-15' });
		expect(rangeFor('12m', new Date(2026, 0, 3))).toEqual({ from: '2025-02-01', to: '2026-01-03' });
	});
	it('fillMonths preenche lacunas, inclusive na virada do ano', () => {
		const out = fillMonths([
			{ month: '2026-01', renewals: 1, successes: 1, failures: 0 },
			{ month: '2025-11', renewals: 2, successes: 1, failures: 1 }
		]);
		expect(out.map((m) => m.month)).toEqual(['2025-11', '2025-12', '2026-01']);
		expect(out[1].renewals).toBe(0);
		expect(fillMonths([])).toEqual([]);
	});
	it('niceScale e taxas', () => {
		expect(niceScale(7)).toEqual({ top: 8, ticks: [0, 2, 4, 6, 8] });
		expect(niceScale(0).top).toBe(1);
		expect(monthRate({ month: 'x', renewals: 0, successes: 0, failures: 0 })).toBeNull();
		expect(formatPercent(0.5)).toMatch(/50\s?%/);
		expect(formatPercent(null)).toBeNull();
	});
});

describe('/estatisticas', () => {
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
		expect(screen.getByRole('img', { name: /Gráfico de colunas com 3 meses, 12 renovações/ })).toBeInTheDocument();
		expect(screen.getByRole('table')).toBeInTheDocument();
		expect(screen.getAllByRole('row')).toHaveLength(4);
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

	it('estado vazio sem renovações', async () => {
		mockIPC((cmd) =>
			cmd === 'stats_overview' ? { ...overview, renewals_total: 0, success_rate: null, most_renewed: null } : []
		);
		render(Page);
		expect(await screen.findByText('Ainda não há renovações')).toBeInTheDocument();
		expect(screen.getByText('Nenhum livro renovado no período.')).toBeInTheDocument();
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
	});
});
