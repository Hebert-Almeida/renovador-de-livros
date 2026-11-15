import '@testing-library/jest-dom/vitest';
import { describe, it, expect, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/svelte';
import { mockCommands, clearMocks } from '#lib/ipc/mock.ts';
import SobrePage from '#lib/features/sobre/SobrePage.svelte';
import { toRows } from '#lib/features/sobre/diagnostics.ts';

afterEach(() => {
	cleanup();
	clearMocks();
});

const entry = {
	id: 1,
	account_id: 2,
	host: 'biblioteca.sophia.com.br',
	at: '2026-11-15T10:00:00Z',
	kind: 'login',
	session_id: 'abc',
	http_status: 200,
	outcome: 'ok'
};

describe('toRows', () => {
	it('formata campos opcionais e não expõe session_id', () => {
		const [r] = toRows([{ ...entry, http_status: null, account_id: null }]);
		expect(r.status).toBe('—');
		expect(r.account).toBe('—');
		expect(Object.keys(r)).not.toContain('session_id');
	});
});

describe('SobrePage', () => {
	it('mostra versão e o registro de requisições', async () => {
		mockCommands({
			app_status: () => ({ version: '2.0.0-alpha.0', accounts: 1, automation: {}, next_run_at: null, onboarding_done: true }),
			diagnostics_requests: (a) => {
				expect(a.limit).toBe(50);
				return [entry];
			}
		});
		render(SobrePage);
		expect(await screen.findByText('2.0.0-alpha.0')).toBeInTheDocument();
		expect(await screen.findByText('biblioteca.sophia.com.br')).toBeInTheDocument();
		expect(screen.getByRole('heading', { level: 1, name: 'Sobre' })).toBeInTheDocument();
		expect(screen.getByText('gsap')).toBeInTheDocument();
	});

	it('mostra estado vazio e erro de versão', async () => {
		mockCommands({
			app_status: () => {
				throw new Error('x');
			},
			diagnostics_requests: () => []
		});
		render(SobrePage);
		expect(await screen.findByText('Indisponível')).toBeInTheDocument();
		expect(await screen.findByText('Nenhuma requisição registrada.')).toBeInTheDocument();
	});

	it('mostra erro do diagnóstico', async () => {
		mockCommands({
			app_status: () => ({ version: '1', accounts: 0, automation: {}, next_run_at: null, onboarding_done: true }),
			diagnostics_requests: () => {
				throw new Error('x');
			}
		});
		render(SobrePage);
		expect(await screen.findByRole('alert')).toHaveTextContent('Não foi possível carregar o registro.');
	});
});
