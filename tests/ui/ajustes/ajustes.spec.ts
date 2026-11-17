import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

// Mock do IPC no navegador: mesmo contrato de `window.__TAURI_INTERNALS__` usado por @tauri-apps/api/mocks.
async function mockIpc(page: Page) {
	await page.addInitScript(() => {
		const w = window as unknown as Record<string, unknown>;
		let settings: Record<string, unknown> = {
			theme: 'light',
			motion: 'system',
			notifications: true,
			autostart: false,
			app_lock: false,
			onboarding_done: true,
			privacy_accepted_at: null
		};
		const calls: { cmd: string; args: unknown }[] = [];
		w.__ipcCalls = calls;
		let cb = 0;
		w.__TAURI_INTERNALS__ = {
			metadata: { currentWindow: { label: 'main' }, currentWebview: { windowLabel: 'main', label: 'main' } },
			transformCallback: () => ++cb,
			unregisterCallback: () => {},
			invoke: async (cmd: string, args: Record<string, unknown>) => {
				calls.push({ cmd, args });
				switch (cmd) {
					case 'settings_get':
						return settings;
					case 'settings_set':
						settings = { ...settings, ...Object.fromEntries(Object.entries(args.patch as object).filter(([, v]) => v != null)) };
						return settings;
					case 'backup_create':
					case 'backup_restore':
					case 'data_wipe':
						return null;
					case 'plugin:dialog|save':
						return null;
					default:
						throw new Error(`comando não mockado: ${cmd}`);
				}
			}
		};
	});
}

async function open(page: Page) {
	await mockIpc(page);
	await page.emulateMedia({ reducedMotion: 'reduce' });
	for (let i = 0; i < 4; i++) {
		await page.goto('/ajustes');
		if (await page.getByRole('heading', { name: 'Backup e dados' }).waitFor({ timeout: 20_000 }).then(() => true, () => false)) break;
	}
	await expect(page.getByRole('heading', { name: 'Backup e dados' })).toBeVisible();
}

const calls = (page: Page) => page.evaluate(() => (window as unknown as { __ipcCalls: { cmd: string; args: unknown }[] }).__ipcCalls);

for (const tema of ['light', 'dark', 'contrast'] as const) {
	test(`/ajustes no tema ${tema} sem violações axe`, async ({ page }) => {
		await open(page);
		await page.getByLabel('Tema', { exact: true }).selectOption(tema);
		await expect(page.locator('html')).toHaveAttribute('data-theme', tema);
		// A troca de tema anima as cores: o axe mede a cor computada e um quadro
		// intermediário dá falso positivo. Repete até estabilizar (violação real persiste).
		const axe = async () => (await new AxeBuilder({ page }).analyze()).violations;
		await expect.poll(axe, { timeout: 10_000 }).toEqual([]);

		await page.getByRole('button', { name: 'Apagar tudo…' }).click();
		await expect(page.getByRole('dialog')).toBeVisible();
		await expect.poll(axe, { timeout: 10_000 }).toEqual([]);
	});
}

test('fluxo completo só por teclado', async ({ page }) => {
	await open(page);

	// Tema via teclado.
	await page.getByLabel('Tema', { exact: true }).focus();
	await page.keyboard.press('ArrowDown'); // light -> dark
	await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');

	// Tab até o switch de iniciar com o sistema.
	const auto = page.getByRole('switch', { name: 'Iniciar com o sistema' });
	for (let i = 0; i < 10 && !(await auto.evaluate((e) => e === document.activeElement)); i++) await page.keyboard.press('Tab');
	await expect(auto).toBeFocused();
	await page.keyboard.press('Space');
	await expect(auto).toHaveAttribute('aria-checked', 'true');

	// Backup: preencher e criar.
	await page.getByLabel('Arquivo de backup').focus();
	await page.keyboard.type('C:\\Backups\\r.bak');
	await page.keyboard.press('Tab');
	await page.keyboard.type('segredo123');
	const create = page.getByRole('button', { name: 'Criar backup' });
	for (let i = 0; i < 6 && !(await create.evaluate((e) => e === document.activeElement)); i++) await page.keyboard.press('Tab');
	await expect(create).toBeFocused();
	await page.keyboard.press('Enter');
	await expect(page.getByRole('status').filter({ hasText: 'Backup criado.' })).toBeVisible();

	// Apagar tudo com confirmação forte.
	const wipe = page.getByRole('button', { name: 'Apagar tudo…' });
	await wipe.focus();
	await page.keyboard.press('Enter');
	await expect(page.getByRole('dialog')).toBeVisible();
	await page.keyboard.press('Escape');
	await expect(page.getByRole('dialog')).toBeHidden();
	await expect(wipe).toBeFocused();
	// Reabre só depois de o diálogo anterior ter fechado de vez.
	await expect(async () => {
		await page.keyboard.press('Enter');
		await expect(page.getByRole('dialog')).toBeVisible({ timeout: 1000 });
	}).toPass({ timeout: 10_000 });
	await page.getByLabel('Digite APAGAR para confirmar').focus();
	await page.keyboard.type('APAGAR');
	await page.keyboard.press('Tab'); // Cancelar
	await page.keyboard.press('Tab'); // Apagar definitivamente
	await expect(page.getByRole('button', { name: 'Apagar definitivamente' })).toBeFocused();
	await page.keyboard.press('Enter');
	await expect(page.getByRole('status').filter({ hasText: 'Todos os dados foram apagados.' })).toBeVisible();

	const cmds = (await calls(page)).map((c) => c.cmd);
	expect(cmds).toContain('backup_create');
	expect(cmds).toContain('data_wipe');
	expect(cmds.filter((c) => c === 'settings_set').length).toBe(2);
});
