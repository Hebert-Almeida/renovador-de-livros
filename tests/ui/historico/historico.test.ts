import { describe, it, expect, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/svelte';
import { mockCommands, clearMocks } from '#lib/ipc/mock.ts';
import HistoryView from '#lib/features/historico/HistoryView.svelte';
import { toasts } from '#lib/components/toast.svelte.ts';
import { toFilter, dueChange, isRangeValid, emptyForm, formatDate } from '#lib/features/historico/history.ts';

afterEach(() => {
	cleanup();
	clearMocks();
	toasts.splice(0, toasts.length);
});

const event = (id: number, over: Record<string, unknown> = {}) => ({
	id,
	loan_id: 10 + id,
	attempted_at: '2026-11-10T12:00:00Z',
	trigger: 'auto',
	result: 'success',
	old_due_date: '2026-11-12',
	new_due_date: '2026-11-19',
	message: null,
	...over
});

function setup(extra: Record<string, (a: Record<string, unknown>) => unknown> = {}) {
	const calls: Record<string, Record<string, unknown>[]> = {};
	const track =
		(name: string, fn: (a: Record<string, unknown>) => unknown) =>
		(a: Record<string, unknown>) => {
			(calls[name] ??= []).push(a);
			return fn(a);
		};
	mockCommands({
		account_list: track('account_list', () => [{ id: 1, login: '123', library_name: 'UFTM' }]),
		history_list: track('history_list', () => ({ items: [event(1), event(2, { result: 'reserved', new_due_date: null })], total: 2 })),
		...Object.fromEntries(Object.entries(extra).map(([k, f]) => [k, track(k, f)]))
	});
	return calls;
}

describe('history.ts', () => {
	it('monta o filtro a partir do formulário', () => {
		expect(toFilter(emptyForm())).toEqual({ account_id: null, loan_id: null, result: null, from: null, to: null });
		const f = toFilter({ account: '3', result: 'captcha', from: '2026-01-01', to: '2026-01-31' });
		expect(f.account_id).toBe(3);
		expect(f.result).toBe('captcha');
		expect(new Date(f.from!).getTime()).toBeLessThan(new Date(f.to!).getTime());
	});
	it('valida o intervalo de datas', () => {
		expect(isRangeValid({ ...emptyForm(), from: '2026-02-01', to: '2026-01-01' })).toBe(false);
		expect(isRangeValid({ ...emptyForm(), from: '2026-01-01', to: '2026-01-01' })).toBe(true);
	});
	it('formata mudança de devolução', () => {
		expect(formatDate('2026-11-12')).toBe('12/11/2026');
		expect(dueChange({ old_due_date: '2026-11-12', new_due_date: '2026-11-19' })).toBe('12/11/2026 → 19/11/2026');
		expect(dueChange({ old_due_date: '2026-11-12', new_due_date: null })).toBe('12/11/2026');
		expect(dueChange({ old_due_date: null, new_due_date: null })).toBe('—');
	});
});

describe('HistoryView', () => {
	it('lista tentativas em pt-BR', async () => {
		setup();
		render(HistoryView);
		expect(await screen.findByText('Nº 11')).toBeInTheDocument();
		expect(screen.getByRole('cell', { name: 'Renovado' })).toBeInTheDocument();
		expect(screen.getByRole('cell', { name: 'Reservado' })).toBeInTheDocument();
		expect(screen.getByText('1–2 de 2')).toBeInTheDocument();
	});

	it('aplica filtros', async () => {
		const calls = setup();
		render(HistoryView);
		await screen.findByText('Nº 11');
		await fireEvent.change(screen.getByLabelText('Resultado'), { target: { value: 'captcha' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Filtrar' }));
		await waitFor(() => expect(calls.history_list).toHaveLength(2));
		expect((calls.history_list[1].filter as { result: string }).result).toBe('captcha');
	});

	it('mostra estado vazio', async () => {
		mockCommands({ account_list: () => [], history_list: () => ({ items: [], total: 0 }) });
		render(HistoryView);
		expect(await screen.findByRole('heading', { name: 'Nenhuma tentativa' })).toBeInTheDocument();
	});

	it('mostra erro de carregamento', async () => {
		mockCommands({
			account_list: () => [],
			history_list: () => {
				throw { kind: 'store', message: 'x' };
			}
		});
		render(HistoryView);
		expect(await screen.findByRole('heading', { name: 'Não foi possível carregar o histórico.' })).toBeInTheDocument();
	});

	it('exporta via diálogo de arquivo e comando IPC', async () => {
		const calls = setup({ 'plugin:dialog|save': () => 'C:/tmp/historico.csv', history_export: () => 2 });
		render(HistoryView);
		await screen.findByText('Nº 11');
		await fireEvent.click(screen.getByRole('button', { name: 'Exportar CSV' }));
		await waitFor(() => expect(calls.history_export).toEqual([{ format: 'csv', path: 'C:/tmp/historico.csv' }]));
		await waitFor(() => expect(toasts.map((t) => t.message)).toContain('2 tentativas exportadas.'));
	});

	it('não exporta se o diálogo for cancelado', async () => {
		const exp = vi.fn();
		setup({ 'plugin:dialog|save': () => null, history_export: exp });
		render(HistoryView);
		await screen.findByText('Nº 11');
		await fireEvent.click(screen.getByRole('button', { name: 'Exportar JSON' }));
		await new Promise((r) => setTimeout(r, 20));
		expect(exp).not.toHaveBeenCalled();
	});

	it('apaga com confirmação', async () => {
		const calls = setup({ history_clear: () => 2 });
		render(HistoryView);
		await screen.findByText('Nº 11');
		await fireEvent.click(screen.getByRole('button', { name: 'Apagar histórico' }));
		expect(calls.history_clear).toBeUndefined();
		await fireEvent.click(screen.getByRole('button', { name: 'Apagar' }));
		await waitFor(() => expect(calls.history_clear).toEqual([{ before: null }]));
		await waitFor(() => expect(toasts.map((t) => t.message)).toContain('2 tentativas apagadas.'));
	});
});
