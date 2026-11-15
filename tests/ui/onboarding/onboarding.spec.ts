import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

// Equivalente no navegador ao `mockIPC` de `@tauri-apps/api/mocks`: define
// `__TAURI_INTERNALS__` antes do app carregar e registra as chamadas.
async function mockIpc(page: Page) {
	await page.addInitScript(() => {
		const w = window as unknown as Record<string, unknown>;
		const calls: { cmd: string; args: unknown }[] = [];
		w.__ipcCalls = calls;
		const handlers: Record<string, (a: Record<string, unknown>) => unknown> = {
			library_presets: () => [{ name: 'UFTM', base_url: 'https://biblioteca.sophia.com.br/5362', supported: true }],
			library_validate: (a) => ({ ok: true, normalized_url: a.url, reason: null }),
			account_create: () => ({ id: 1 }),
			account_update: () => ({ id: 1 }),
			account_test_login: () => true,
			settings_set: () => ({})
		};
		let cb = 0;
		w.__TAURI_INTERNALS__ = {
			invoke: async (cmd: string, args: Record<string, unknown>) => {
				calls.push({ cmd, args });
				const h = handlers[cmd];
				if (!h) throw { kind: 'not_implemented', message: cmd };
				return h(args ?? {});
			},
			transformCallback: () => ++cb,
			metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } }
		};
	});
	await page.context().grantPermissions(['notifications']);
}

async function open(page: Page) {
	for (let i = 0; i < 4; i++) {
		await page.goto('/onboarding');
		if (await page.locator('h1').waitFor({ timeout: 20_000 }).then(() => true, () => false)) break;
	}
	await expect(page.locator('h1')).toHaveText('Boas-vindas');
}

async function axe(page: Page) {
	const res = await new AxeBuilder({ page }).analyze();
	expect(res.violations).toEqual([]);
}

async function tabTo(page: Page, name: RegExp | string, role: 'button' | 'checkbox' | 'switch' | 'textbox' | 'radio') {
	const target = page.getByRole(role, { name }).first();
	for (let i = 0; i < 40; i++) {
		if (await target.evaluate((el) => el === document.activeElement).catch(() => false)) return;
		await page.keyboard.press('Tab');
	}
	throw new Error(`não alcançou ${String(name)} pelo teclado`);
}

test('fluxo completo só por teclado, com axe zerado em cada etapa', async ({ page }) => {
	await page.emulateMedia({ reducedMotion: 'reduce' });
	await mockIpc(page);
	await open(page);
	await axe(page);

	await tabTo(page, /Li e concordo/, 'checkbox');
	await page.keyboard.press('Space');
	await tabTo(page, 'Continuar', 'button');
	await page.keyboard.press('Enter');

	await expect(page.getByRole('heading', { name: 'Escolha a biblioteca' })).toBeFocused();
	await expect(page.getByRole('radio', { name: /UFTM/ })).toBeChecked();
	await tabTo(page, /Avançado/, 'switch');
	await page.keyboard.press('Space');
	await expect(page.getByRole('note')).toContainText('experimentais');
	await axe(page);
	await page.keyboard.press('Space');
	await tabTo(page, 'Continuar', 'button');
	await page.keyboard.press('Enter');

	await expect(page.getByRole('heading', { name: 'Sua conta na biblioteca' })).toBeFocused();
	await tabTo(page, 'Matrícula ou login', 'textbox');
	await page.keyboard.type('2020123');
	await page.keyboard.press('Tab');
	await page.keyboard.type('segredo');
	await tabTo(page, /Automático/, 'radio');
	await page.keyboard.press('ArrowDown');
	await expect(page.getByRole('radio', { name: /Manual/ })).toBeChecked();
	await axe(page);
	await page.keyboard.press('Enter');

	await expect(page.getByRole('heading', { name: 'Teste de login' })).toBeFocused();
	await tabTo(page, 'Testar login', 'button');
	await page.keyboard.press('Enter');
	await expect(page.getByText('Login confirmado.')).toBeVisible();
	await axe(page);
	await tabTo(page, 'Continuar', 'button');
	await page.keyboard.press('Enter');

	await expect(page.getByRole('heading', { name: 'Notificações' })).toBeFocused();
	await tabTo(page, 'Permitir notificações', 'button');
	await page.keyboard.press('Enter');
	await expect(page.getByText('Notificações permitidas.')).toBeVisible();
	await axe(page);
	await tabTo(page, 'Concluir', 'button');
	await page.keyboard.press('Enter');

	await page.waitForURL((u) => u.pathname === '/');
	const calls = (await page.evaluate(() => (window as unknown as { __ipcCalls: { cmd: string; args: unknown }[] }).__ipcCalls)) ?? [];
	const set = calls.find((c) => c.cmd === 'settings_set');
	expect(set?.args).toMatchObject({ patch: { onboarding_done: true, notifications: true } });
	const create = calls.find((c) => c.cmd === 'account_create');
	expect(create?.args).toMatchObject({ account: { login: '2020123', mode: 'manual' } });
});

for (const tema of ['dark', 'contrast'] as const) {
	test(`axe zerado no tema ${tema}`, async ({ page }) => {
		await page.emulateMedia({ reducedMotion: 'reduce' });
		await mockIpc(page);
		await open(page);
		await page.evaluate((t) => (document.documentElement.dataset.theme = t), tema);
		await axe(page);
	});
}
