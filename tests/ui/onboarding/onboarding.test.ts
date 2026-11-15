import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/svelte';
import { mockCommands, clearMocks } from '#lib/ipc/mock.ts';
import { defaultPreset, errorMessage, validateLibrary, saveOnly, UFTM_FALLBACK } from '#lib/features/onboarding/flow.ts';
import OnboardingWizard from '#lib/features/onboarding/OnboardingWizard.svelte';

afterEach(() => {
	cleanup();
	clearMocks();
});

const presets = [
	{ name: 'Outra', base_url: 'https://x.sophia.com.br/1', supported: false },
	{ name: 'UFTM', base_url: 'https://biblioteca.sophia.com.br/5362', supported: true }
];

describe('flow', () => {
	it('pré-seleciona a biblioteca suportada', () => {
		expect(defaultPreset(presets).name).toBe('UFTM');
		expect(defaultPreset([])).toEqual(UFTM_FALLBACK);
	});

	it('traduz erros do backend', () => {
		expect(errorMessage({ kind: 'invalid_credentials' })).toMatch(/incorretas/);
		expect(errorMessage({ kind: 'guard', message: { reason: 'quota_exhausted', retry_at: null } })).toMatch(/cota/);
		expect(errorMessage('x')).toMatch(/errado/);
	});

	it('valida endereço customizado', async () => {
		mockCommands({ library_validate: ({ url }) => ({ ok: true, normalized_url: `${url}/`, reason: null }) });
		expect(await validateLibrary(' https://a ')).toEqual({ ok: true, url: 'https://a/' });
		expect((await validateLibrary('  ')).ok).toBe(false);
	});

	it('cria a conta na primeira vez e atualiza depois', async () => {
		const create = vi.fn(() => ({ id: 7 }));
		const update = vi.fn(() => ({ id: 7 }));
		mockCommands({ account_create: create, account_update: update });
		const draft = { libraryUrl: 'u', libraryName: 'n', login: ' 123 ', password: 'p', mode: 'auto' as const };
		expect(await saveOnly(draft, null)).toBe(7);
		expect(create).toHaveBeenCalledOnce();
		expect((create.mock.calls[0] as unknown[])[0]).toMatchObject({ account: { login: '123', mode: 'auto' } });
		await saveOnly(draft, 7);
		expect(update).toHaveBeenCalledOnce();
	});
});

describe('OnboardingWizard', () => {
	it('percorre o fluxo completo', async () => {
		const settings = vi.fn(() => ({}));
		mockCommands({
			library_presets: () => presets,
			account_create: () => ({ id: 1 }),
			account_test_login: () => true,
			settings_set: settings
		});
		const onDone = vi.fn();
		render(OnboardingWizard, { onDone });
		const next = () => fireEvent.click(screen.getByRole('button', { name: 'Continuar' }));

		await next();
		expect(await screen.findByText(/marque a caixa/)).toBeInTheDocument();
		await fireEvent.click(screen.getByRole('checkbox'));
		await next();

		const uftm = await screen.findByRole('radio', { name: /UFTM/ });
		expect(uftm).toBeChecked();
		await next();

		await screen.findByRole('heading', { name: 'Sua conta na biblioteca' });
		await fireEvent.input(screen.getByLabelText('Matrícula ou login'), { target: { value: '123' } });
		await fireEvent.input(screen.getByLabelText('Senha da biblioteca'), { target: { value: 'x' } });
		await fireEvent.click(screen.getByRole('radio', { name: /Manual/ }));
		await next();

		await fireEvent.click(await screen.findByRole('button', { name: 'Testar login' }));
		expect(await screen.findByText('Login confirmado.')).toBeInTheDocument();
		await next();

		await screen.findByRole('heading', { name: 'Notificações' });
		await fireEvent.click(screen.getByRole('button', { name: 'Concluir' }));
		await waitFor(() => expect(onDone).toHaveBeenCalled());
		expect((settings.mock.calls[0] as unknown[])[0]).toMatchObject({
			patch: { onboarding_done: true, notifications: false }
		});
	});

	it('modo avançado mostra aviso e valida o endereço', async () => {
		mockCommands({
			library_presets: () => presets,
			library_validate: () => ({ ok: false, normalized_url: null, reason: 'Não é Sophia.' })
		});
		render(OnboardingWizard);
		await fireEvent.click(screen.getByRole('checkbox'));
		await fireEvent.click(screen.getByRole('button', { name: 'Continuar' }));
		await fireEvent.click(await screen.findByRole('switch', { name: /Avançado/ }));
		expect(screen.getByRole('note')).toHaveTextContent(/experimentais/);
		await fireEvent.input(screen.getByLabelText(/Endereço da biblioteca/), { target: { value: 'https://e' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Continuar' }));
		expect(await screen.findByText('Não é Sophia.')).toBeInTheDocument();
	});
});
