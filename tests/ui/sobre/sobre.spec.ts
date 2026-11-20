import { test, expect, type Page } from '@playwright/test';
import { semViolacoesAxe } from '../axe.ts';

// Mock do IPC do Tauri no navegador (equivalente ao mockIPC de @tauri-apps/api/mocks).
async function mockIpc(page: Page) {
	await page.addInitScript(() => {
		const handlers: Record<string, () => unknown> = {
			app_status: () => ({ version: '2.0.0-alpha.0', accounts: 1, automation: {}, next_run_at: null, onboarding_done: true }),
			diagnostics_trace_status: () => ({ enabled: true, path: 'C:/dados/diagnostico', pages: 2 }),
			diagnostics_requests: () => [
				{ id: 1, account_id: 1, host: 'biblioteca.sophia.com.br', at: '2026-11-15T10:00:00Z', kind: 'login', session_id: 's', http_status: 200, outcome: 'ok' }
			]
		};
		const w = window as unknown as Record<string, unknown>;
		w.__TAURI_INTERNALS__ = {
			invoke: async (cmd: string) => {
				const h = handlers[cmd];
				if (!h) throw new Error(`comando não mockado: ${cmd}`);
				return h();
			},
			transformCallback: () => 0,
			metadata: { currentWindow: { label: 'main' }, currentWebview: { windowLabel: 'main', label: 'main' } }
		};
	});
}

async function open(page: Page) {
	await mockIpc(page);
	await page.emulateMedia({ reducedMotion: 'reduce' });
	for (let i = 0; i < 4; i++) {
		await page.goto('/sobre');
		if (await page.getByTestId('app-version').waitFor({ timeout: 20_000 }).then(() => true, () => false)) break;
	}
	await expect(page.getByTestId('app-version')).toHaveText('2.0.0-alpha.0');
	await expect(page.getByRole('cell', { name: 'biblioteca.sophia.com.br' })).toBeVisible();
}

for (const tema of ['light', 'dark', 'contrast'] as const) {
	test(`/sobre no tema ${tema} sem violações axe`, async ({ page }) => {
		await open(page);
		await page.evaluate((t) => document.documentElement.setAttribute('data-theme', t), tema);
		await semViolacoesAxe(page);
		await page.getByRole('tab', { name: 'Uso responsável' }).click();
		await semViolacoesAxe(page);
	});
}

test('teclado: abas legais, atualizar diagnóstico', async ({ page }) => {
	await open(page);
	const tabPriv = page.getByRole('tab', { name: 'Aviso de privacidade' });
	for (let i = 0; i < 60 && !(await tabPriv.evaluate((el) => el === document.activeElement)); i++) {
		await page.keyboard.press('Tab');
	}
	await expect(tabPriv).toBeFocused();
	await page.keyboard.press('ArrowRight');
	const tabUso = page.getByRole('tab', { name: 'Uso responsável' });
	await expect(tabUso).toBeFocused();
	await expect(tabUso).toHaveAttribute('aria-selected', 'true');
	await expect(page.getByText('Renovações automáticas só entre 07h e 21h.')).toBeVisible();

	const refresh = page.getByRole('button', { name: 'Atualizar registro' });
	for (let i = 0; i < 30 && !(await refresh.evaluate((el) => el === document.activeElement)); i++) {
		await page.keyboard.press('Tab');
	}
	await expect(refresh).toBeFocused();
	await page.keyboard.press('Enter');
	await expect(page.getByRole('status').filter({ hasText: '1 requisição carregada.' })).toBeAttached();
	await expect(page.getByRole('cell', { name: 'biblioteca.sophia.com.br' })).toBeVisible();
});
