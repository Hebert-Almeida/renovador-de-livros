import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/svelte';
import { mockCommands, clearMocks } from '#lib/ipc/mock.ts';
import { formatDate, daysUntil, loanPill, coverFor, sortEvents, errorMessage } from '#lib/features/emprestimos/loans.ts';
import type { LoanView_Serialize, RenewalEvent_Serialize } from '#lib/ipc/index.ts';

const params = vi.hoisted(() => ({ id: '1' }));
vi.mock('$app/state', () => ({ page: { params } }));

import ListPage from '../../../src/routes/emprestimos/+page.svelte';
import DetailPage from '../../../src/routes/emprestimos/[id]/+page.svelte';

const view = (id: number, title: string, due: string, status: 'active' | 'overdue' | 'returned' = 'active'): LoanView_Serialize => ({
	loan: { id, book_id: id, loaned_at: '2026-10-01', due_date: due, renewals_count: 1, renewals_limit: 5, status, auto_renew: true, returned_at: null, updated_at: '2026-10-01T00:00:00Z' },
	book: { id, account_id: 7, item_key: `k${id}`, title, author: 'Machado de Assis', call_number: '869.3', library_branch: 'Central', first_seen_at: '2026-10-01' }
});
const ev = (id: number, at: string, result: RenewalEvent_Serialize['result'] = 'success'): RenewalEvent_Serialize => ({
	id, loan_id: 1, attempted_at: at, trigger: 'auto', result, old_due_date: null, new_due_date: '2026-12-01', message: null
});

afterEach(() => {
	cleanup();
	clearMocks();
});

describe('regras', () => {
	it('formata datas e conta dias', () => {
		expect(formatDate('2026-11-20T10:00:00Z')).toBe('20/11/2026');
		expect(formatDate(null)).toBe('—');
		expect(daysUntil('2026-11-20', new Date(2026, 10, 15))).toBe(5);
		expect(daysUntil('2026-11-14', new Date(2026, 10, 15))).toBe(-1);
	});
	it('pílula de situação', () => {
		const today = new Date(2026, 10, 15);
		expect(loanPill(view(1, 'A', '2026-11-30'), today)).toEqual({ status: 'ok', label: 'Vence em 15 dias' });
		expect(loanPill(view(1, 'A', '2026-11-16'), today).status).toBe('warn');
		expect(loanPill(view(1, 'A', '2026-11-10'), today).label).toBe('Atrasado há 5 dias');
		expect(loanPill(view(1, 'A', '2026-11-30', 'returned'), today).label).toBe('Devolvido');
	});
	it('capa determinística a partir do título', () => {
		expect(coverFor('Dom Casmurro')).toEqual(coverFor('Dom Casmurro'));
		expect(coverFor('Dom Casmurro').initials).toBe('DC');
		expect(coverFor('Vidas Secas').hue).toBeGreaterThanOrEqual(0);
	});
	it('ordena carimbos e descreve erros', () => {
		expect(sortEvents([ev(2, '2026-11-02'), ev(1, '2026-10-01')]).map((e) => e.id)).toEqual([1, 2]);
		expect(errorMessage({ kind: 'network', message: 'timeout' })).toContain('timeout');
	});
});

describe('/emprestimos', () => {
	it('lista lombadas e filtra por situação', async () => {
		const calls: unknown[] = [];
		mockCommands({
			loan_list: ({ filter }) => {
				calls.push(filter);
				return (filter as { status: string | null }).status === 'overdue' ? [] : [view(1, 'Dom Casmurro', '2026-12-20')];
			}
		});
		render(ListPage);
		expect(await screen.findByRole('link', { name: /Dom Casmurro/ })).toHaveAttribute('href', '/emprestimos/1');
		await fireEvent.change(screen.getByLabelText('Situação'), { target: { value: 'overdue' } });
		expect(await screen.findByRole('heading', { name: 'Nada encontrado' })).toBeInTheDocument();
		expect(calls.at(-1)).toEqual({ account_id: null, status: 'overdue', search: null });
	});
	it('mostra erro com nova tentativa', async () => {
		mockCommands({ loan_list: () => Promise.reject({ kind: 'store', message: 'trancado' }) });
		render(ListPage);
		expect(await screen.findByRole('alert')).toHaveTextContent('trancado');
	});
});

describe('/emprestimos/[id]', () => {
	beforeEach(() => (params.id = '1'));
	it('mostra ficha, carimbos e alterna a renovação automática', async () => {
		const set = vi.fn(() => null);
		mockCommands({
			loan_get: () => ({ view: view(1, 'Dom Casmurro', '2026-12-20'), events: [ev(1, '2026-11-02T10:00:00Z')], schedules: [] }),
			loan_set_auto_renew: set
		});
		render(DetailPage);
		expect(await screen.findByRole('heading', { level: 2, name: 'Dom Casmurro' })).toBeInTheDocument();
		expect(screen.getAllByText(/Renovado/).length).toBeGreaterThan(0);
		const sw = screen.getByRole('switch', { name: 'Renovação automática' });
		await fireEvent.click(sw);
		await waitFor(() => expect(set).toHaveBeenCalledWith({ id: 1, on: false }));
		expect(sw).toHaveAttribute('aria-checked', 'false');
	});
	it('desfaz o switch se o IPC falhar', async () => {
		mockCommands({
			loan_get: () => ({ view: view(1, 'Dom Casmurro', '2026-12-20'), events: [], schedules: [] }),
			loan_set_auto_renew: () => Promise.reject({ kind: 'store', message: 'x' })
		});
		render(DetailPage);
		const sw = await screen.findByRole('switch', { name: 'Renovação automática' });
		await fireEvent.click(sw);
		await waitFor(() => expect(sw).toHaveAttribute('aria-checked', 'true'));
	});
	it('marca como devolvido após confirmar', async () => {
		let status: 'active' | 'returned' = 'active';
		mockCommands({
			loan_get: () => ({ view: view(1, 'Dom Casmurro', '2026-12-20', status), events: [], schedules: [] }),
			loan_mark_returned: () => {
				status = 'returned';
				return null;
			}
		});
		render(DetailPage);
		await fireEvent.click(await screen.findByRole('button', { name: 'Marcar como devolvido' }));
		await fireEvent.click(screen.getByRole('button', { name: 'Confirmar' }));
		await waitFor(() => expect(screen.queryByRole('button', { name: 'Renovar agora' })).not.toBeInTheDocument());
		expect(screen.getAllByText('Devolvido').length).toBeGreaterThan(0);
	});
	it('não encontrado', async () => {
		mockCommands({ loan_get: () => Promise.reject({ kind: 'not_found', message: 'loan 9' }) });
		params.id = '9';
		render(DetailPage);
		expect(await screen.findByRole('alert')).toHaveTextContent('Empréstimo não encontrado.');
	});
});
