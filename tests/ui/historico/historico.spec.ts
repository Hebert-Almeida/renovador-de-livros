import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

// Mock do IPC do Tauri no navegador. Equivale ao `mockIPC` de `@tauri-apps/api/mocks`,
// que não pode ser importado dentro de `addInitScript`.
async function mockIpc(page: Page) {
	await page.addInitScript(() => {
		const w = window as unknown as Record<string, unknown>;
		const calls: { cmd: string; args: unknown }[] = [];
		w.__calls = calls;
		let items = Array.from({ length: 3 }, (_, i) => ({
			id: i + 1,
			loan_id: 100 + i,
			attempted_at: `2026-11-1${i}T12:00:00Z`,
			trigger: i === 0 ? 'manual' : 'auto',
			result: i === 2 ? 'reserved' : 'success',
			old_due_date: '2026-11-12',
			new_due_date: i === 2 ? null : '2026-11-19',
			message: i === 2 ? 'Livro reservado por outro leitor' : null
		}));
		const handlers: Record<string, (a: Record<string, unknown>) => unknown> = {
			account_list: () => [{ id: 1, login: '2023001', library_name: 'UFTM' }],
			history_list: (a) => {
				const f = a.filter as { result: string | null };
				const list = f.result ? items.filter((e) => e.result === f.result) : items;
				return { items: list, total: list.length };
			},
			'plugin:dialog|save': () => 'C:/tmp/historico.csv',
			history_export: () => items.length,
			history_clear: () => {
				const n = items.length;
				items = [];
				return n;
			}
		};
		w.__TAURI_INTERNALS__ = {
			metadata: { currentWindow: { label: 'main' }, currentWebview: { windowLabel: 'main', label: 'main' } },
			transformCallback: () => 0,
			invoke: async (cmd: string, args: Record<string, unknown>) => {
				calls.push({ cmd, args });
				const h = handlers[cmd];
				if (!h) throw new Error(`comando não mockado: ${cmd}`);
				return h(args ?? {});
			}
		};
	});
}

async function open(page: Page) {
	for (let i = 0; i < 4; i++) {
		await page.goto('/historico');
		if (await page.getByRole('cell', { name: 'Nº 100' }).waitFor({ timeout: 20_000 }).then(() => true, () => false)) break;
	}
	await expect(page.locator('h1')).toHaveText('Histórico');
}

test.beforeEach(async ({ page }) => {
	await page.emulateMedia({ reducedMotion: 'reduce' });
	await mockIpc(page);
});

test('/historico sem violações axe (lista e diálogo)', async ({ page }) => {
	await open(page);
	expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
	await page.getByRole('button', { name: 'Apagar histórico' }).click();
	await expect(page.getByRole('dialog')).toBeVisible();
	expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
});

test('fluxo completo só por teclado: filtrar, exportar e apagar', async ({ page }) => {
	await open(page);

	// Filtrar por resultado.
	const result = page.getByLabel('Resultado');
	await result.focus();
	await page.keyboard.press('ArrowDown'); // Renovado
	await page.keyboard.press('ArrowDown'); // Reservado
	await expect(result).toHaveValue('reserved');
	await page.keyboard.press('Tab');
	await expect(page.getByLabel('De', { exact: true })).toBeFocused();
	await page.keyboard.press('Enter'); // envio implícito do formulário de filtros
	await expect(page.getByRole('cell', { name: 'Nº 100' })).toBeHidden();
	await expect(page.getByRole('cell', { name: 'Nº 102' })).toBeVisible();

	// Limpar filtros.
	await page.getByRole('button', { name: 'Limpar filtros' }).focus();
	await page.keyboard.press('Enter');
	await expect(page.getByRole('cell', { name: 'Nº 100' })).toBeVisible();

	// Exportar CSV.
	await page.keyboard.press('Tab');
	await expect(page.getByRole('button', { name: 'Exportar CSV' })).toBeFocused();
	await page.keyboard.press('Enter');
	await expect(page.getByRole('status').filter({ hasText: '3 tentativas exportadas.' })).toBeVisible();
	const exp = await page.evaluate(() =>
		((window as unknown as { __calls: { cmd: string; args: unknown }[] }).__calls).find((c) => c.cmd === 'history_export')
	);
	expect(exp?.args).toMatchObject({ format: 'csv', path: 'C:/tmp/historico.csv' });

	// Apagar: abre, cancela com Esc (foco volta), reabre e confirma.
	const clear = page.getByRole('button', { name: 'Apagar histórico' });
	await clear.focus();
	await page.keyboard.press('Enter');
	await expect(page.getByRole('dialog')).toBeVisible();
	await page.keyboard.press('Escape');
	await expect(page.getByRole('dialog')).toBeHidden();
	await expect(clear).toBeFocused();
	await page.keyboard.press('Enter');
	await expect(page.getByRole('dialog')).toBeVisible();
	await page.getByRole('dialog').getByRole('button', { name: 'Apagar', exact: true }).focus();
	await page.keyboard.press('Enter');
	await expect(page.getByRole('heading', { name: 'Nenhuma tentativa' })).toBeVisible();
	await expect(page.getByRole('status').filter({ hasText: '3 tentativas apagadas.' })).toBeVisible();
});
