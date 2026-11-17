import { test, expect, type Page } from '@playwright/test';
import { semViolacoesAxe } from '../axe.ts';

// Mock do IPC do Tauri no navegador (equivalente a mockIPC de @tauri-apps/api/mocks).
async function mockIpc(page: Page) {
	await page.addInitScript(() => {
		const book = (id: number, title: string, author: string) => ({
			id, account_id: 1, item_key: `k${id}`, title, author, call_number: '869.3', library_branch: 'Central', first_seen_at: '2026-10-01'
		});
		const today = new Date();
		const plus = (d: number) => new Date(today.getTime() + d * 86_400_000).toISOString().slice(0, 10);
		const db = {
			loans: [
				{ id: 1, book_id: 1, loaned_at: '2026-10-01', due_date: plus(10), renewals_count: 1, renewals_limit: 5, status: 'active', auto_renew: true, returned_at: null, updated_at: '' },
				{ id: 2, book_id: 2, loaned_at: '2026-10-01', due_date: plus(-2), renewals_count: 0, renewals_limit: 5, status: 'overdue', auto_renew: false, returned_at: null, updated_at: '' },
				{ id: 3, book_id: 3, loaned_at: '2026-10-01', due_date: plus(1), renewals_count: 5, renewals_limit: 5, status: 'active', auto_renew: true, returned_at: null, updated_at: '' }
			] as Record<string, unknown>[],
			books: [book(1, 'Dom Casmurro', 'Machado de Assis'), book(2, 'Vidas Secas', 'Graciliano Ramos'), book(3, 'Grande Sertão: Veredas', 'Guimarães Rosa')],
			events: [{ id: 1, loan_id: 1, attempted_at: '2026-10-20T09:00:00Z', trigger: 'auto', result: 'success', old_due_date: null, new_due_date: plus(10), message: null }] as Record<string, unknown>[]
		};
		const viewOf = (l: Record<string, unknown>) => ({ loan: l, book: db.books.find((b) => b.id === l.book_id) });
		const handlers: Record<string, (a: Record<string, any>) => unknown> = {
			loan_list: ({ filter }) =>
				db.loans
					.filter((l) => !filter.status || l.status === filter.status)
					.map(viewOf)
					.filter((v) => !filter.search || `${v.book!.title} ${v.book!.author}`.toLowerCase().includes(filter.search.toLowerCase())),
			loan_get: ({ id }) => {
				const l = db.loans.find((x) => x.id === id);
				if (!l) throw { kind: 'not_found', message: String(id) };
				return { view: viewOf(l), events: db.events.filter((e) => e.loan_id === id), schedules: [] };
			},
			loan_set_auto_renew: ({ id, on }) => {
				db.loans.find((x) => x.id === id)!.auto_renew = on;
				return null;
			},
			loan_mark_returned: ({ id }) => {
				const l = db.loans.find((x) => x.id === id)!;
				l.status = 'returned';
				l.returned_at = today.toISOString();
				return null;
			},
			loan_renew: () => {
				const e = { id: db.events.length + 1, loan_id: 1, attempted_at: today.toISOString(), trigger: 'manual', result: 'success', old_due_date: null, new_due_date: plus(17), message: null };
				db.events.push(e);
				const l = db.loans[0];
				l.due_date = plus(17);
				l.renewals_count = (l.renewals_count as number) + 1;
				return { account_id: 1, requests_made: 3, loans_seen: 3, renewed: [e], failed: [], returned_detected: 0, error: null };
			}
		};
		let cb = 0;
		(window as any).__TAURI_INTERNALS__ = {
			invoke: async (cmd: string, args: Record<string, unknown>) => {
				const h = handlers[cmd];
				if (!h) throw new Error(`comando não mockado: ${cmd}`);
				return h(args ?? {});
			},
			transformCallback: () => ++cb,
			unregisterCallback: () => {},
			metadata: { currentWindow: { label: 'main' }, currentWebview: { windowLabel: 'main', label: 'main' } }
		};
	});
}

async function open(page: Page, url: string, heading: RegExp | string) {
	for (let i = 0; i < 4; i++) {
		await page.goto(url);
		if (await page.locator('h1').first().waitFor({ timeout: 20_000 }).then(() => true, () => false)) break;
	}
	await expect(page.getByRole('heading', { level: 1, name: heading })).toBeAttached();
}

test.beforeEach(async ({ page }) => {
	await page.emulateMedia({ reducedMotion: 'reduce' });
	await mockIpc(page);
});

for (const tema of ['light', 'dark'] as const) {
	test(`axe sem violações (${tema})`, async ({ page }) => {
		await page.emulateMedia({ reducedMotion: 'reduce', colorScheme: tema });
		await open(page, '/emprestimos', 'Empréstimos');
		await expect(page.getByRole('link', { name: /Dom Casmurro/ })).toBeVisible();
		await semViolacoesAxe(page);

		await open(page, '/emprestimos/1', 'Dom Casmurro');
		await expect(page.getByRole('switch', { name: 'Renovação automática' })).toBeVisible();
		await semViolacoesAxe(page);

		await page.getByRole('button', { name: 'Marcar como devolvido' }).click();
		await expect(page.getByRole('dialog')).toBeVisible();
		await semViolacoesAxe(page);
	});
}

test('fluxo completo só por teclado', async ({ page }) => {
	await open(page, '/emprestimos', 'Empréstimos');
	await expect(page.getByRole('link', { name: /Dom Casmurro/ })).toBeVisible();

	// Filtro por situação.
	const sel = page.getByLabel('Situação');
	await sel.focus();
	await sel.selectOption('overdue'); // select nativo: equivale a setas no teclado
	await expect(page.getByRole('link', { name: /Vidas Secas/ })).toBeVisible();
	await expect(page.getByRole('link', { name: /Dom Casmurro/ })).toHaveCount(0);
	await sel.selectOption('');

	// Busca.
	const busca = page.getByLabel('Buscar por título ou autor');
	await busca.focus();
	await page.keyboard.type('casmurro');
	await expect(page.getByRole('link', { name: /Vidas Secas/ })).toHaveCount(0);

	// Tab até a lombada e Enter.
	const link = page.getByRole('link', { name: /Dom Casmurro/ });
	for (let i = 0; i < 10 && !(await link.evaluate((el) => el === document.activeElement)); i++) await page.keyboard.press('Tab');
	await expect(link).toBeFocused();
	await page.keyboard.press('Enter');
	await expect(page).toHaveURL(/\/emprestimos\/1$/);

	// Liga/desliga a renovação automática.
	const sw = page.getByRole('switch', { name: 'Renovação automática' });
	await sw.focus();
	await page.keyboard.press('Space');
	await expect(sw).toHaveAttribute('aria-checked', 'false');
	await expect(page.getByRole('status').filter({ hasText: 'Renovação automática desligada.' })).toBeVisible();

	// Renovar agora: novo carimbo.
	await page.keyboard.press('Tab');
	await expect(page.getByRole('button', { name: 'Renovar agora' })).toBeFocused();
	await page.keyboard.press('Enter');
	await expect(page.getByRole('status').filter({ hasText: 'Renovação concluída.' })).toBeVisible();
	await expect(page.getByText('2 de 5')).toBeVisible();
	await expect(page.locator('.log li')).toHaveCount(2);

	// Marcar como devolvido, com confirmação.
	const devolver = page.getByRole('button', { name: 'Marcar como devolvido' });
	await devolver.focus();
	await page.keyboard.press('Enter');
	await expect(page.getByRole('dialog')).toBeVisible();
	await page.keyboard.press('Escape');
	await expect(page.getByRole('dialog')).toBeHidden();
	await expect(devolver).toBeFocused();
	// Reabre só depois de o diálogo anterior ter fechado de vez.
	await expect(async () => {
		await page.keyboard.press('Enter');
		await expect(page.getByRole('dialog')).toBeVisible({ timeout: 1000 });
	}).toPass({ timeout: 10_000 });
	await page.getByRole('button', { name: 'Confirmar' }).focus();
	await page.keyboard.press('Enter');
	await expect(page.getByRole('status').filter({ hasText: 'Empréstimo marcado como devolvido.' })).toBeVisible();
	await expect(page.getByRole('button', { name: 'Renovar agora' })).toHaveCount(0);

	// Volta à lista.
	await page.getByRole('link', { name: 'Voltar para empréstimos' }).focus();
	await page.keyboard.press('Enter');
	await expect(page).toHaveURL(/\/emprestimos$/);
});

test('alvos interativos têm pelo menos 44px', async ({ page }) => {
	for (const url of ['/emprestimos', '/emprestimos/1']) {
		await open(page, url, /.+/);
		await expect(page.locator('main a, main button').first()).toBeVisible();
		await page.waitForTimeout(500);
		const small = await page.$$eval('main button, main a, main input, main select', (els) =>
			els
				.filter((e) => (e as HTMLElement).offsetParent !== null)
				.map((e) => ({ e: e.outerHTML.slice(0, 60), r: e.getBoundingClientRect() }))
				.filter(({ r }) => r.width < 44 || r.height < 44)
				.map(({ e }) => e)
		);
		expect(small).toEqual([]);
	}
});
