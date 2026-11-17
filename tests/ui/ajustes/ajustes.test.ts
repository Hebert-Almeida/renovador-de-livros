import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/svelte';
import Page from '../../../src/routes/ajustes/+page.svelte';
import { mockCommands, clearMocks } from '#lib/ipc/mock.ts';
import { applyTheme, applyMotion, errorText, passphraseOk } from '#lib/features/ajustes/settings.ts';
import { prefersReducedMotion } from '#lib/motion/index.ts';

const base = {
	theme: 'light',
	motion: 'system',
	notifications: true,
	autostart: false,
	app_lock: false,
	onboarding_done: true,
	privacy_accepted_at: null
};

let current: Record<string, unknown>;
let calls: { cmd: string; args: Record<string, unknown> }[];

beforeEach(() => {
	current = { ...base };
	calls = [];
	const log = (cmd: string, fn: (a: Record<string, unknown>) => unknown) => (a: Record<string, unknown>) => {
		calls.push({ cmd, args: a });
		return fn(a);
	};
	mockCommands({
		settings_get: log('settings_get', () => current),
		settings_set: log('settings_set', (a) => {
			const patch = a.patch as Record<string, unknown>;
			current = { ...current, ...patch };
			return current;
		}),
		backup_create: log('backup_create', () => null),
		backup_restore: log('backup_restore', () => null),
		data_wipe: log('data_wipe', () => null)
	});
});

afterEach(() => {
	cleanup();
	clearMocks();
	delete document.documentElement.dataset.theme;
	delete document.documentElement.dataset.motion;
});

describe('settings (lógica)', () => {
	it('aplica e remove tema/movimento', () => {
		applyTheme('dark');
		expect(document.documentElement.dataset.theme).toBe('dark');
		applyTheme('system');
		expect(document.documentElement.dataset.theme).toBeUndefined();
		applyMotion('reduced');
		expect(document.documentElement.dataset.motion).toBe('reduced');
		applyMotion('system');
		expect(document.documentElement.dataset.motion).toBeUndefined();
	});
	it('lib/motion obedece a escolha de movimento dos Ajustes', () => {
		applyMotion('reduced');
		expect(prefersReducedMotion()).toBe(true);
		applyMotion('full');
		expect(prefersReducedMotion()).toBe(false);
		applyMotion('system');
	});
	it('valida senha e extrai erros', () => {
		expect(passphraseOk('1234567')).toBe(false);
		expect(passphraseOk('12345678')).toBe(true);
		expect(errorText({ kind: 'store', message: 'falhou' })).toBe('falhou');
		expect(errorText({ kind: 'invalid_credentials' })).toBe('invalid_credentials');
	});
});

describe('/ajustes', () => {
	it('carrega e aplica o tema salvo', async () => {
		render(Page);
		await screen.findByRole('heading', { name: 'Aparência e movimento' });
		expect(document.documentElement.dataset.theme).toBe('light');
		expect(screen.getByRole('switch', { name: 'Notificações' })).toHaveAttribute('aria-checked', 'true');
	});

	it('troca o tema e persiste via IPC', async () => {
		render(Page);
		const sel = (await screen.findByLabelText('Tema')) as HTMLSelectElement;
		await fireEvent.change(sel, { target: { value: 'contrast' } });
		await waitFor(() => expect(current.theme).toBe('contrast'));
		expect(document.documentElement.dataset.theme).toBe('contrast');
	});

	it('alterna iniciar com o sistema', async () => {
		render(Page);
		const sw = await screen.findByRole('switch', { name: 'Iniciar com o sistema' });
		await fireEvent.click(sw);
		await waitFor(() => expect(current.autostart).toBe(true));
	});

	it('reverte o switch quando salvar falha', async () => {
		mockCommands({
			settings_get: () => current,
			settings_set: () => {
				throw { kind: 'store', message: 'x' };
			}
		});
		render(Page);
		const sw = await screen.findByRole('switch', { name: 'Bloquear o app com a senha mestra' });
		await fireEvent.click(sw);
		await waitFor(() => expect(sw).toHaveAttribute('aria-checked', 'false'));
	});

	it('valida e cria backup', async () => {
		render(Page);
		const create = await screen.findByRole('button', { name: 'Criar backup' });
		await fireEvent.click(create);
		expect(await screen.findByText(/Informe o arquivo de backup./)).toBeInTheDocument();
		expect(calls.some((c) => c.cmd === 'backup_create')).toBe(false);
		await fireEvent.input(screen.getByLabelText('Arquivo de backup', { exact: false }), { target: { value: 'C:\b.bak' } });
		await fireEvent.input(screen.getByLabelText('Senha do backup', { exact: false }), { target: { value: 'segredo123' } });
		await fireEvent.click(create);
		await waitFor(() => expect(calls.find((c) => c.cmd === 'backup_create')?.args).toEqual({ path: 'C:\b.bak', passphrase: 'segredo123' }));
	});

	it('apagar tudo exige digitar APAGAR', async () => {
		render(Page);
		await fireEvent.click(await screen.findByRole('button', { name: 'Apagar tudo…' }));
		const confirm = screen.getByRole('button', { name: 'Apagar definitivamente', hidden: true });
		expect(confirm).toBeDisabled();
		await fireEvent.input(screen.getByLabelText('Digite APAGAR para confirmar'), { target: { value: 'APAGAR' } });
		expect(confirm).toBeEnabled();
		await fireEvent.click(confirm);
		await waitFor(() => expect(calls.some((c) => c.cmd === 'data_wipe')).toBe(true));
	});
});
