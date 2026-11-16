import { describe, it, expect, afterEach } from 'vitest';
import { render, screen, fireEvent, cleanup, waitFor, within } from '@testing-library/svelte';
import AgendaView from '#lib/features/agenda/AgendaView.svelte';
import { mockCommands, clearMocks } from '#lib/ipc/mock.ts';
import type { Schedule_Serialize } from '#lib/ipc/bindings.ts';
import { actionsFor, fromLocalInput, toLocalInput, validateRunAt } from '#lib/features/agenda/schedule.ts';

afterEach(() => {
	cleanup();
	clearMocks();
});

const base = (over: Partial<Schedule_Serialize>): Schedule_Serialize => ({
	id: 1,
	account_id: 1,
	loan_id: 10,
	kind: 'renew',
	run_at: '2099-03-10T12:00:00.000Z',
	status: 'pending',
	origin: 'system',
	updated_at: '2099-03-01T12:00:00.000Z',
	...over
});

function setup(initial: Schedule_Serialize[]) {
	const db = new Map(initial.map((s) => [s.id, { ...s }]));
	const calls: [string, Record<string, unknown>][] = [];
	const set = (id: number, patch: Partial<Schedule_Serialize>) => {
		const s = { ...db.get(id)!, ...patch };
		db.set(id, s);
		return s;
	};
	mockCommands({
		schedule_list: () => [...db.values()],
		schedule_pause: (a) => (calls.push(['pause', a]), set(a.id as number, { status: 'paused' })),
		schedule_reactivate: (a) => (calls.push(['reactivate', a]), set(a.id as number, { status: 'pending' })),
		schedule_cancel: (a) => (calls.push(['cancel', a]), set(a.id as number, { status: 'cancelled' })),
		schedule_reschedule: (a) => (calls.push(['reschedule', a]), set(a.id as number, { run_at: a.runAt as string }))
	});
	return calls;
}

describe('regras da agenda', () => {
	it('converte entre ISO UTC e horário de São Paulo', () => {
		expect(toLocalInput('2099-03-10T12:00:00.000Z')).toBe('2099-03-10T09:00');
		expect(fromLocalInput('2099-03-10T09:00')).toBe('2099-03-10T12:00:00.000Z');
		expect(fromLocalInput('lixo')).toBeNull();
	});
	it('usa a base de fusos, não um deslocamento fixo (horário de verão de 2018)', () => {
		expect(toLocalInput('2018-12-01T12:00:00.000Z')).toBe('2018-12-01T10:00');
		expect(fromLocalInput('2018-12-01T10:00')).toBe('2018-12-01T12:00:00.000Z');
	});
	it('valida a janela 07:00–21:00 e o futuro', () => {
		const now = new Date('2099-01-01T00:00:00Z');
		expect(validateRunAt('2099-03-10T07:00', now)).toBeNull();
		expect(validateRunAt('2099-03-10T20:59', now)).toBeNull();
		expect(validateRunAt('2099-03-10T06:59', now)).toBe('window');
		expect(validateRunAt('2099-03-10T21:00', now)).toBe('window');
		expect(validateRunAt('2000-03-10T10:00', now)).toBe('past');
		expect(validateRunAt('', now)).toBe('invalid');
	});
	it('ações por estado', () => {
		expect(actionsFor('pending')).toEqual(['pause', 'reschedule', 'cancel']);
		expect(actionsFor('paused')).toContain('reactivate');
		expect(actionsFor('done')).toEqual([]);
	});
});

describe('AgendaView', () => {
	it('lista vazia mostra estado vazio', async () => {
		setup([]);
		render(AgendaView);
		expect(await screen.findByRole('heading', { name: 'Nada agendado' })).toBeInTheDocument();
	});

	it('pausa e reativa', async () => {
		const calls = setup([base({})]);
		render(AgendaView);
		await fireEvent.click(await screen.findByRole('button', { name: /^Pausar: Renovação/ }));
		await waitFor(() => expect(screen.getByText('Pausado')).toBeInTheDocument());
		await fireEvent.click(screen.getByRole('button', { name: /^Reativar: Renovação/ }));
		await waitFor(() => expect(screen.getByText('Agendado')).toBeInTheDocument());
		expect(calls.map((c) => c[0])).toEqual(['pause', 'reactivate']);
	});

	it('cancelar renovação agendada pede confirmação', async () => {
		const calls = setup([base({})]);
		render(AgendaView);
		await fireEvent.click(await screen.findByRole('button', { name: /^Cancelar renovação agendada:/ }));
		const dlg = screen.getByRole('dialog', { name: 'Cancelar renovação agendada?' });
		await fireEvent.click(within(dlg).getByRole('button', { name: 'Sim, cancelar' }));
		await waitFor(() => expect(screen.getByText('Cancelado')).toBeInTheDocument());
		expect(calls).toEqual([['cancel', { id: 1 }]]);
	});

	it('reagendar fora da janela é recusado; dentro, envia em UTC', async () => {
		const calls = setup([base({ kind: 'sync', loan_id: null })]);
		render(AgendaView);
		await fireEvent.click(await screen.findByRole('button', { name: /^Reagendar: Sincronização/ }));
		const input = screen.getByLabelText('Nova data e hora (horário de Brasília)');
		await fireEvent.input(input, { target: { value: '2099-03-11T22:00' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Salvar' }));
		expect(await screen.findByText(/Fora da janela permitida/)).toBeInTheDocument();
		expect(calls).toEqual([]);
		await fireEvent.input(input, { target: { value: '2099-03-11T08:30' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Salvar' }));
		await waitFor(() => expect(calls).toEqual([['reschedule', { id: 1, runAt: '2099-03-11T11:30:00.000Z' }]]));
	});

	it('erro ao carregar oferece nova tentativa', async () => {
		mockCommands({
			schedule_list: () => {
				throw new Error('x');
			}
		});
		render(AgendaView);
		expect(await screen.findByRole('alert')).toHaveTextContent('Não foi possível carregar a agenda.');
	});
});
