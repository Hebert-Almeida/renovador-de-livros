import { test, expect, type Page } from '@playwright/test';
import { semViolacoesAxe } from '../axe.ts';

const temas = ['light', 'dark', 'contrast'] as const;

// Na primeira carga o Vite (dev) pode reotimizar dependências e recarregar; tenta de novo.
async function open(page: Page, url: string) {
	for (let i = 0; i < 4; i++) {
		await page.goto(url);
		if (await page.locator('h1').waitFor({ timeout: 20_000 }).then(() => true, () => false)) break;
	}
	await expect(page.locator('h1')).toHaveText('Kit de componentes');
}

for (const tema of temas) {
	test(`/dev/kit no tema ${tema} sem violações axe`, async ({ page }) => {
		await page.emulateMedia({ reducedMotion: 'reduce' });
		await open(page, `/dev/kit?tema=${tema}`);
		await expect(page.locator('html')).toHaveAttribute('data-theme', tema);
		await semViolacoesAxe(page);

		// Aba "Vazio" e diálogo aberto também passam no axe.
		await page.getByRole('tab', { name: 'Vazio' }).click();
		await page.getByRole('button', { name: 'Abrir diálogo' }).click();
		await expect(page.getByRole('dialog')).toBeVisible();
		await semViolacoesAxe(page);
	});
}

test('teclado: abas, switch, diálogo e toast', async ({ page }) => {
	await page.emulateMedia({ reducedMotion: 'reduce' });
	await open(page, '/dev/kit');
	const tab = page.getByRole('tab', { name: 'Tabela' });
	await tab.focus();
	await page.keyboard.press('ArrowRight');
	await expect(page.getByRole('tab', { name: 'Vazio' })).toBeFocused();
	await expect(page.getByRole('heading', { name: 'Nenhum empréstimo' })).toBeVisible();

	const sw = page.getByRole('switch', { name: 'Renovação automática' });
	await sw.focus();
	await page.keyboard.press('Space');
	await expect(sw).toHaveAttribute('aria-checked', 'false');

	const opener = page.getByRole('button', { name: 'Abrir diálogo' });
	await opener.focus();
	await page.keyboard.press('Enter');
	await expect(page.getByRole('dialog')).toBeVisible();
	await page.keyboard.press('Escape');
	await expect(page.getByRole('dialog')).toBeHidden();
	await expect(opener).toBeFocused();

	await page.getByRole('button', { name: 'Renovar agora' }).focus();
	await page.keyboard.press('Enter');
	await expect(page.getByRole('status').filter({ hasText: 'Empréstimo renovado.' })).toBeVisible();
});

test('alvos interativos têm pelo menos 44px', async ({ page }) => {
	await open(page, '/dev/kit');
	const small = await page.$$eval('main button, main a, main input, main select', (els) =>
		els
			.filter((e) => (e as HTMLElement).offsetParent !== null)
			.map((e) => ({ e: e.outerHTML.slice(0, 60), r: e.getBoundingClientRect() }))
			.filter(({ r }) => r.width < 44 || r.height < 44)
			.map(({ e }) => e)
	);
	expect(small).toEqual([]);
});
