import { describe, it, expect, afterEach, vi } from 'vitest';
import { render, screen, cleanup, fireEvent, waitFor } from '@testing-library/svelte';
import { mockIPC, clearMocks } from '@tauri-apps/api/mocks';
import { emit } from '@tauri-apps/api/event';
import Painel from '#lib/features/painel/Painel.svelte';
import { daysUntil, dueLabel, dueTone, errorMessage, formatDate, quotaView } from '#lib/features/painel/format.ts';
import { account, appStatus, loans, quota } from './fixtures.ts';

const NOW = new Date(2026, 10, 15, 10, 0);

afterEach(() => {
	cleanup();
	clearMocks();
	vi.restoreAllMocks();
});

describe('format', () => {
	it('calcula dias e rótulos de vencimento', () => {
		expect(daysUntil('2026-11-15', NOW)).toBe(0);
		expect(dueLabel('2026-11-16', NOW)).toBe('vence amanhã');
		expect(dueLabel('2026-11-20', NOW)).toBe('vence em 5 dias');
		expect(dueLabel('2026-11-13', NOW)).toBe('atrasado 2 dias');
		expect(dueTone('2026-11-17', NOW)).toBe('warn');
		expect(dueTone('2026-11-14', NOW)).toBe('danger');
		expect(dueTone('2026-12-01', NOW)).toBe('ok');
		expect(formatDate('2026-11-20')).toBe('20/11/2026');
	});

	it('libera ou bloqueia a sincronização pela cota e espera', () => {
		expect(quotaView(quota({ manual_sessions_left: 2 }), NOW)).toEqual({
			canSync: true,
			text: 'Restam 2 sincronizações manuais hoje.'
		});
		const later = new Date(NOW.getTime() + 30 * 60_000).toISOString();
		const wait = quotaView(quota({ manual_sessions_left: 1, next_allowed_at: later }), NOW);
		expect(wait.canSync).toBe(false);
		expect(wait.text).toMatch(/Resta 1 sincronização manual hoje\. Disponível de novo às \d{2}:\d{2}\./);
		expect(quotaView(quota({ manual_sessions_left: 0 }), NOW)).toEqual({
			canSync: false,
			text: 'A cota de sincronizações manuais de hoje acabou.'
		});
		const breaker = quotaView(quota({ breaker_open_until: later }), NOW);
		expect(breaker.canSync).toBe(false);
	});

	it('traduz erros do guard', () => {
		expect(errorMessage({ kind: 'guard', message: { reason: 'quota_exhausted', retry_at: null } })).toBe(
			'A cota de sincronizações manuais de hoje acabou.'
		);
		expect(errorMessage({ kind: 'invalid_credentials' })).toBe('Login ou senha da biblioteca incorretos.');
		expect(errorMessage('x')).toBe('A sincronização falhou.');
	});
});

type Handlers = Record<string, (args: Record<string, unknown>) => unknown>;
function mock(h: Handlers) {
	mockIPC(
		(cmd, args) => {
			const fn = h[cmd];
			if (!fn) throw new Error(`comando não mockado: ${cmd}`);
			return fn((args ?? {}) as Record<string, unknown>);
		},
		{ shouldMockEvents: true }
	);
}

const base: Handlers = {
	app_status: () => appStatus(),
	account_list: () => [account()],
	loan_list: () => loans(),
	quota_status: () => quota()
};

describe('Painel', () => {
	it('mostra próximos vencimentos ordenados, automação e cota', async () => {
		mock(base);
		render(Painel);
		const list = await screen.findByRole('list', { name: 'Empréstimos ativos por data de devolução' });
		const items = list.querySelectorAll('li');
		expect(items).toHaveLength(2);
		expect(items[0]).toHaveTextContent('Vidas Secas');
		expect(screen.getByText('Ligada')).toBeInTheDocument();
		expect(screen.getByText('Restam 2 sincronizações manuais hoje.')).toBeInTheDocument();
		expect(screen.queryByText('Dom Casmurro (devolvido)')).toBeNull();
	});

	it('sem contas, mostra o estado vazio com o HeroBook', async () => {
		mock({ ...base, account_list: () => [], loan_list: () => [] });
		const { container } = render(Painel);
		expect(await screen.findByRole('heading', { name: 'Comece cadastrando sua conta' })).toBeInTheDocument();
		expect(container.querySelector('.hero-book')).not.toBeNull();
		expect(screen.getByRole('link', { name: 'Cadastrar conta' })).toHaveAttribute('href', '/contas');
	});

	it('sem empréstimos, mostra o estado vazio', async () => {
		mock({ ...base, loan_list: () => [] });
		render(Painel);
		expect(await screen.findByRole('heading', { name: 'Nenhum empréstimo por aqui' })).toBeInTheDocument();
	});

	it('"Sincronizar agora" anuncia o progresso e o resultado no aria-live', async () => {
		let release!: () => void;
		const gate = new Promise<void>((r) => (release = r));
		const run = vi.fn(async () => {
			await gate;
			return { account_id: 1, requests_made: 3, loans_seen: 2, renewed: [], failed: [], returned_detected: 0, error: null };
		});
		mock({ ...base, cycle_run_now: run });
		const { container } = render(Painel);
		const btn = await screen.findByRole('button', { name: 'Sincronizar agora' });
		await fireEvent.click(btn);
		expect(run).toHaveBeenCalledWith({ accountId: 1, kind: 'sync' });
		const live = container.querySelector('.painel__live')!;
		expect(live).toHaveAttribute('aria-live', 'polite');
		await emit('cycle-progress-event', { account_id: 1, step: 'list', message: 'Lendo empréstimos', current: 1, total: 2 });
		const steps = screen.getByRole('list', { name: 'Progresso da sincronização' });
		expect(steps).toBeInTheDocument();
		// O progresso é anunciado uma vez só, pela região do ProgressSteps.
		const stepsLive = steps.parentElement!.querySelector('[aria-live]')!;
		await waitFor(() => expect(stepsLive).toHaveTextContent('em andamento'));
		expect(live).not.toHaveTextContent('Lendo empréstimos');
		release();
		await waitFor(() =>
			expect(live).toHaveTextContent('Sincronização concluída: 2 empréstimos, 0 renovados.')
		);
	});

	it('bloqueia o botão quando a cota acabou e explica o erro do guard', async () => {
		mock({ ...base, quota_status: () => quota({ manual_sessions_left: 0 }) });
		render(Painel);
		const btn = await screen.findByRole('button', { name: 'Sincronizar agora' });
		expect(btn).toBeDisabled();
		expect(screen.getByText('A cota de sincronizações manuais de hoje acabou.')).toBeInTheDocument();
	});
});
