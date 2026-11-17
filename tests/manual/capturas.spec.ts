// Gera as capturas de tela do manual (docs/manual/img/*.png) com o backend de demonstração.
import { test, expect, type Page } from '@playwright/test';
import { installDemoBackend } from './demo-backend.ts';

const OUT = 'docs/manual/img';

async function open(page: Page, path: string, heading: string | RegExp, onboarded = true) {
	await page.clock.setFixedTime(new Date('2026-11-17T13:00:00Z'));
	await page.emulateMedia({ reducedMotion: 'reduce', colorScheme: 'light' });
	await page.addInitScript(installDemoBackend, onboarded);
	// Na primeira carga o Vite (dev) pode reotimizar dependências e recarregar; tenta de novo.
	for (let i = 0; i < 4; i++) {
		await page.goto(path);
		if (await page.locator('h1').waitFor({ timeout: 20_000 }).then(() => true, () => false)) break;
	}
	await expect(page.locator('h1')).toHaveText(heading);
	await page.waitForLoadState('networkidle');
	await page.waitForTimeout(600);
}

const shot = (page: Page, name: string, fullPage = false) => page.screenshot({ path: `${OUT}/${name}.png`, fullPage });

test('onboarding', async ({ page }) => {
	await open(page, '/onboarding', 'Boas-vindas', false);
	await shot(page, 'onboarding-privacidade');
	await page.getByRole('checkbox').check();
	await page.getByRole('button', { name: 'Continuar' }).click();
	await expect(page.getByRole('heading', { name: 'Escolha a biblioteca' })).toBeVisible();
	await page.getByRole('button', { name: 'Continuar' }).click();
	await expect(page.getByRole('heading', { name: 'Sua conta na biblioteca' })).toBeVisible();
	await page.waitForTimeout(300);
	await shot(page, 'onboarding-conta');
});

test('painel', async ({ page }) => {
	await open(page, '/', 'Painel');
	await expect(page.getByRole('heading', { name: 'Próximos vencimentos' })).toBeVisible();
	await shot(page, 'painel', true);
});

test('empréstimos e ficha', async ({ page }) => {
	await open(page, '/emprestimos', 'Empréstimos');
	await shot(page, 'emprestimos', true);
	await open(page, '/emprestimos/2', 'Dom Casmurro');
	await shot(page, 'emprestimo-ficha', true);
});

test('agenda', async ({ page }) => {
	await open(page, '/agenda', 'Agenda');
	await shot(page, 'agenda', true);
});

test('histórico', async ({ page }) => {
	await open(page, '/historico', 'Histórico');
	await shot(page, 'historico', true);
});

test('estatísticas', async ({ page }) => {
	await open(page, '/estatisticas', 'Estatísticas');
	await expect(page.getByText('Livro mais renovado')).toBeVisible();
	await shot(page, 'estatisticas', true);
});

test('contas', async ({ page }) => {
	await open(page, '/contas', 'Contas');
	await shot(page, 'contas');
	await page.getByRole('button', { name: 'Editar' }).first().click();
	await expect(page.getByRole('dialog')).toBeVisible();
	await page.waitForTimeout(400);
	await shot(page, 'contas-editar');
});

test('ajustes', async ({ page }) => {
	await open(page, '/ajustes', 'Ajustes');
	await shot(page, 'ajustes', true);
});

test('sobre', async ({ page }) => {
	await open(page, '/sobre', 'Sobre');
	await shot(page, 'sobre');
});
