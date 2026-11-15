import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { installFakeBackend } from './fake-backend.ts';

// Equivale ao `mockIPC` de @tauri-apps/api/mocks, injetado antes do app carregar.
async function mockTauri(page: Page, seed: number) {
	await page.addInitScript(
		({ src, seed }) => {
			// eslint-disable-next-line no-new-func
			const handler = new Function(`return (${src})`)()(seed);
			let cb = 0;
			(window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
				invoke: async (cmd: string, args: Record<string, unknown>) => handler(cmd, args),
				transformCallback: () => ++cb,
				unregisterCallback: () => {},
				metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main', windowLabel: 'main' } }
			};
		},
		{ src: installFakeBackend.toString(), seed }
	);
}

async function open(page: Page) {
	for (let i = 0; i < 4; i++) {
		await page.goto('/contas');
		if (await page.locator('h1').filter({ hasText: 'Contas' }).waitFor({ timeout: 20_000 }).then(() => true, () => false)) break;
	}
	await expect(page.getByRole('heading', { level: 1, name: 'Contas' })).toBeVisible();
}

async function axe(page: Page) {
	const res = await new AxeBuilder({ page }).analyze();
	expect(res.violations).toEqual([]);
}

async function tabTo(page: Page, name: string | RegExp, role: 'button' | 'switch' | 'textbox' = 'button') {
	const target = page.getByRole(role, { name, exact: typeof name === 'string' });
	for (let i = 0; i < 60; i++) {
		if (await target.evaluate((el) => el === document.activeElement).catch(() => false)) return;
		await page.keyboard.press('Tab');
	}
	throw new Error(`não alcançou ${name} pelo teclado`);
}

test.beforeEach(async ({ page }) => {
	await page.emulateMedia({ reducedMotion: 'reduce' });
});

test('estado vazio e diálogos sem violações axe', async ({ page }) => {
	await mockTauri(page, 0);
	await open(page);
	await expect(page.getByRole('heading', { name: 'Nenhuma conta cadastrada' })).toBeVisible();
	await axe(page);
	await page.getByRole('button', { name: 'Adicionar conta' }).click();
	await expect(page.getByRole('dialog')).toBeVisible();
	await axe(page);
});

test('lista cheia sem violações axe', async ({ page }) => {
	await mockTauri(page, 3);
	await open(page);
	await expect(page.getByText('3 de 3 contas')).toBeVisible();
	await axe(page);
	await page.getByRole('button', { name: 'Excluir 100' }).click();
	await expect(page.getByRole('dialog')).toBeVisible();
	await axe(page);
});

test('fluxo completo só por teclado', async ({ page }) => {
	await mockTauri(page, 0);
	await open(page);
	await expect(page.getByRole('heading', { name: 'Nenhuma conta cadastrada' })).toBeVisible();

	// Criar
	await tabTo(page, 'Adicionar conta');
	await page.keyboard.press('Enter');
	await expect(page.getByRole('dialog')).toBeVisible();
	await tabTo(page, /Matrícula/, 'textbox');
	await page.keyboard.type('2024001');
	await page.keyboard.press('Tab');
	await page.keyboard.type('segredo');
	await page.keyboard.press('Enter');
	await expect(page.getByRole('heading', { name: '2024001' })).toBeVisible();
	await expect(page.locator('body')).not.toContainText('segredo');

	// Editar: trocar senha
	await tabTo(page, 'Editar 2024001');
	await page.keyboard.press('Enter');
	await expect(page.getByRole('dialog')).toBeVisible();
	await tabTo(page, 'Nova senha', 'textbox');
	await page.keyboard.type('outra');
	await tabTo(page, 'Salvar');
	await page.keyboard.press('Enter');
	await expect(page.getByRole('status').filter({ hasText: 'Conta atualizada.' })).toBeVisible();

	// Testar login (mostra a cota)
	await tabTo(page, 'Testar login 2024001');
	await page.keyboard.press('Enter');
	await expect(page.getByText('Sessões manuais restantes hoje: 3.')).toBeVisible();
	await tabTo(page, 'Testar e consumir cota');
	await page.keyboard.press('Enter');
	await expect(page.getByRole('status').filter({ hasText: 'Login confirmado.' })).toBeVisible();

	// Excluir com confirmação explícita
	await tabTo(page, 'Excluir 2024001');
	await page.keyboard.press('Enter');
	await expect(page.getByRole('button', { name: 'Excluir e apagar dados' })).toBeDisabled();
	await tabTo(page, 'Entendo que os dados desta conta serão apagados', 'switch');
	await page.keyboard.press('Space');
	await tabTo(page, 'Excluir e apagar dados');
	await page.keyboard.press('Enter');
	await expect(page.getByRole('heading', { name: 'Nenhuma conta cadastrada' })).toBeVisible();
	await expect(page.getByRole('button', { name: 'Adicionar conta' })).toBeFocused();
});
