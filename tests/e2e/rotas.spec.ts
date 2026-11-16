import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

const rotas = ['/', '/onboarding', '/emprestimos', '/agenda', '/historico', '/estatisticas', '/contas', '/ajustes', '/sobre'];

for (const r of rotas) {
	test(`rota ${r} abre sem violações axe`, async ({ page }) => {
		await page.goto(r);
		await expect(page.locator('h1')).toBeVisible();
		// Parte do conteúdo (ex.: estado de erro do IPC) entra com fade depois do h1; o axe mede a cor
		// computada, então um quadro intermediário dá falso positivo de contraste. Repetir a análise até
		// estabilizar: uma violação real persiste e reprova no prazo.
		await expect
			.poll(async () => (await new AxeBuilder({ page }).analyze()).violations, { timeout: 10_000 })
			.toEqual([]);
	});
}
