import { describe, it, expect, afterEach, beforeEach } from 'vitest';
import { render, screen, fireEvent, cleanup, waitFor, within } from '@testing-library/svelte';
import { mockIPC, clearMocks } from '@tauri-apps/api/mocks';
import Page from '../../../src/routes/contas/+page.svelte';
import ToastRegion from '#lib/components/ToastRegion.svelte';
import { describeError } from '#lib/features/contas/errors.ts';
import { installFakeBackend } from './fake-backend.ts';

type Call = { cmd: string; args?: Record<string, unknown> };
const calls = () => (globalThis as unknown as { __contasCalls: Call[] }).__contasCalls;

function setup(seed: number) {
	const handler = installFakeBackend(seed);
	mockIPC((cmd, args) => handler(cmd, args as Record<string, unknown>));
	render(ToastRegion);
	return render(Page);
}

const openDialog = () =>
	waitFor(() => {
		const d = document.querySelector<HTMLElement>('dialog[open]');
		if (!d) throw new Error('nenhum diálogo aberto');
		return d;
	});

beforeEach(() => clearMocks());
afterEach(() => cleanup());

describe('/contas', () => {
	it('mostra estado vazio e cria conta sem expor a senha', async () => {
		setup(0);
		await screen.findByRole('heading', { name: 'Nenhuma conta cadastrada' });
		await fireEvent.click(screen.getByRole('button', { name: 'Adicionar conta' }));
		const dialog = await openDialog();
		await fireEvent.input(within(dialog).getByLabelText('Matrícula', { exact: false }), { target: { value: '2024001' } });
		await fireEvent.input(within(dialog).getByLabelText('Senha da biblioteca'), { target: { value: 'segredo' } });
		await fireEvent.click(within(dialog).getByRole('button', { name: 'Salvar', hidden: true }));
		await screen.findByRole('heading', { name: '2024001' });
		const create = calls().find((c) => c.cmd === 'account_create');
		expect((create?.args?.account as { password: string }).password).toBe('segredo');
		// Padrões da seção 5.4: renova 1 dia antes, lembra 2 dias antes.
		expect(create?.args?.account).toMatchObject({ renew_days_before: 1, remind_days_before: 2 });
		expect(document.body.textContent).not.toContain('segredo');
		expect(screen.getByText('1 de 3 contas')).toBeInTheDocument();
	});

	it('exige matrícula', async () => {
		setup(0);
		await screen.findByRole('heading', { name: 'Nenhuma conta cadastrada' });
		await fireEvent.click(screen.getByRole('button', { name: 'Adicionar conta' }));
		const dialog = await openDialog();
		await fireEvent.click(within(dialog).getByRole('button', { name: 'Salvar', hidden: true }));
		expect(await within(dialog).findByText(/Campo obrigatório/)).toBeInTheDocument();
		expect(calls().some((c) => c.cmd === 'account_create')).toBe(false);
	});

	it('bloqueia a 4ª conta', async () => {
		setup(3);
		await screen.findByText('3 de 3 contas');
		const add = screen.getByRole('button', { name: 'Adicionar conta' });
		expect(add).toHaveAttribute('aria-disabled', 'true');
		await fireEvent.click(add);
		expect(document.querySelector('dialog[open]')).toBeNull();
	});

	it('teste de login mostra a cota antes de consumir', async () => {
		setup(1);
		await fireEvent.click(await screen.findByRole('button', { name: 'Testar login 100' }));
		expect(await screen.findByText('Sessões manuais restantes hoje: 3.')).toBeInTheDocument();
		expect(calls().some((c) => c.cmd === 'account_test_login')).toBe(false);
		await fireEvent.click(screen.getByRole('button', { name: 'Testar e consumir cota', hidden: true }));
		await screen.findByText('Login confirmado.');
	});

	it('exclusão exige confirmação explícita', async () => {
		setup(1);
		await fireEvent.click(await screen.findByRole('button', { name: 'Excluir 100' }));
		const confirm = screen.getByRole('button', { name: 'Excluir e apagar dados', hidden: true });
		expect(confirm).toBeDisabled();
		await fireEvent.click(screen.getByRole('switch', { hidden: true }));
		expect(confirm).toBeEnabled();
		await fireEvent.click(confirm);
		await screen.findByRole('heading', { name: 'Nenhuma conta cadastrada' });
		expect(calls().find((c) => c.cmd === 'account_delete')?.args).toEqual({ id: 1 });
	});

	it('trocar senha envia só a nova senha', async () => {
		setup(1);
		await fireEvent.click(await screen.findByRole('button', { name: 'Editar 100' }));
		const dialog = await openDialog();
		await fireEvent.input(within(dialog).getByLabelText('Nova senha'), { target: { value: 'nova' } });
		await fireEvent.click(within(dialog).getByRole('button', { name: 'Salvar', hidden: true }));
		await waitFor(() => expect(calls().some((c) => c.cmd === 'account_update')).toBe(true));
		const patch = calls().find((c) => c.cmd === 'account_update')?.args?.patch as Record<string, unknown>;
		expect(patch.password).toBe('nova');
		expect(patch.clear_password).toBe(false);
	});

	it('ajusta a antecedência de renovação e de lembrete da conta', async () => {
		setup(1);
		await fireEvent.click(await screen.findByRole('button', { name: 'Editar 100' }));
		const dialog = await openDialog();
		await fireEvent.change(within(dialog).getByLabelText('Renovar', { exact: false }), { target: { value: '3' } });
		await fireEvent.change(within(dialog).getByLabelText('Lembrete', { exact: false }), { target: { value: '0' } });
		await fireEvent.click(within(dialog).getByRole('button', { name: 'Salvar', hidden: true }));
		await waitFor(() => expect(calls().some((c) => c.cmd === 'account_update')).toBe(true));
		const patch = calls().find((c) => c.cmd === 'account_update')?.args?.patch as Record<string, unknown>;
		expect(patch).toMatchObject({ renew_days_before: 3, remind_days_before: 0 });
		expect(await screen.findByText('renova 3 d antes · lembra no dia')).toBeInTheDocument();
	});
});

describe('describeError', () => {
	it('traduz erros do IPC', () => {
		expect(describeError({ kind: 'invalid_credentials' })).toBe('Matrícula ou senha incorretas.');
		expect(describeError({ kind: 'guard', message: { reason: 'account_limit' } })).toBe('Limite de 3 contas atingido.');
		expect(describeError(new Error('x'))).toBe('Algo deu errado. Tente de novo.');
	});
});
