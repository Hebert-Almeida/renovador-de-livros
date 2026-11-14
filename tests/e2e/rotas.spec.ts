import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

const rotas = ['/', '/onboarding', '/emprestimos', '/agenda', '/historico', '/estatisticas', '/contas', '/ajustes', '/sobre'];

for (const r of rotas) {
	test(`rota ${r} abre sem violações axe`, async ({ page }) => {
		await page.goto(r);
		await expect(page.locator('h1')).toBeVisible();
		const res = await new AxeBuilder({ page }).analyze();
		expect(res.violations).toEqual([]);
	});
}
