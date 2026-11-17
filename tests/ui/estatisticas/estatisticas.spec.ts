import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { overview, monthly } from './fixtures.ts';

const temas = ['light', 'dark', 'contrast'] as const;

// Mock do IPC do Tauri no navegador: mesma forma que `@tauri-apps/api/mocks` (mockIPC) instala.
async function mock(page: Page, theme: string) {
	await page.addInitScript(
		({ overview, monthly, theme }) => {
			const w = window as unknown as Record<string, any>;
			w.__TAURI_INTERNALS__ = w.__TAURI_INTERNALS__ ?? {};
			w.__TAURI_INTERNALS__.transformCallback = (cb: (x: unknown) => void) => {
				const id = Math.floor(Math.random() * 1e9);
				w[`_${id}`] = cb;
				return id;
			};
			w.__TAURI_INTERNALS__.invoke = async (cmd: string) => {
				if (cmd === 'stats_overview') return overview;
				if (cmd === 'stats_monthly') return monthly;
				if (cmd.startsWith('plugin:event')) return 0;
				return null;
			};
			w.__TAURI_INTERNALS__.metadata = { currentWindow: { label: 'main' }, currentWebview: { windowLabel: 'main', label: 'main' } };
			document.addEventListener('DOMContentLoaded', () => (document.documentElement.dataset.theme = theme));
		},
		{ overview, monthly, theme }
	);
}

async function open(page: Page) {
	for (let i = 0; i < 4; i++) {
		await page.goto('/estatisticas');
		if (await page.getByText('Livros distintos').waitFor({ timeout: 20_000 }).then(() => true, () => false)) break;
	}
	await expect(page.getByRole('heading', { level: 1, name: 'Estatísticas' })).toBeVisible();
}

for (const tema of temas) {
	test(`/estatisticas no tema ${tema} sem violações axe`, async ({ page }) => {
		await page.emulateMedia({ reducedMotion: 'reduce' });
		await mock(page, tema);
		await open(page);
		await expect(page.getByRole('img', { name: /Gráfico de colunas/ })).toBeVisible();
		await expect(page.locator('html')).toHaveAttribute('data-theme', tema);
		const res = await new AxeBuilder({ page }).analyze();
		expect(res.violations).toEqual([]);
	});
}

test('teclado: período, tabela e ordenação', async ({ page }) => {
	await page.emulateMedia({ reducedMotion: 'reduce' });
	await mock(page, 'light');
	await open(page);

	const select = page.getByLabel('Período');
	for (let i = 0; i < 40 && !(await select.evaluate((el) => el === document.activeElement)); i++) {
		await page.keyboard.press('Tab');
	}
	await expect(select).toBeFocused();
	await page.keyboard.press('ArrowDown');
	await expect(select).toHaveValue('all');
	// a recarga não desmonta o filtro: o foco continua nele
	await expect(select).toBeFocused();
	await expect(page.getByText('Livros distintos')).toBeVisible();

	const sortBtn = page.getByRole('button', { name: /Tentativas/ });
	for (let i = 0; i < 40 && !(await sortBtn.evaluate((el) => el === document.activeElement)); i++) {
		await page.keyboard.press('Tab');
	}
	await expect(sortBtn).toBeFocused();
	await page.keyboard.press('Enter');
	await expect(page.locator('th[aria-sort="ascending"]')).toContainText('Tentativas');
	await page.keyboard.press('Enter');
	await expect(page.locator('th[aria-sort="descending"]')).toContainText('Tentativas');
	const first = page.locator('tbody tr').first();
	await expect(first).toContainText('9');
});
