import { test, expect, type Page } from '@playwright/test';
import { semViolacoesAxe } from '../axe.ts';

// No navegador não há Tauri: instala um IPC falso no mesmo formato que `mockIPC`
// (`@tauri-apps/api/mocks`) usa, `window.__TAURI_INTERNALS__.invoke`.
async function mockIpc(page: Page) {
	await page.addInitScript(() => {
		const db = new Map<number, Record<string, unknown>>();
		const mk = (id: number, kind: string, status: string, run_at: string, loan_id: number | null) => ({
			id, account_id: 1, loan_id, kind, run_at, status, origin: 'system', updated_at: run_at
		});
		[
			mk(1, 'renew', 'pending', '2099-03-10T12:00:00.000Z', 10),
			mk(2, 'sync', 'paused', '2099-03-11T13:00:00.000Z', null),
			mk(3, 'remind', 'done', '2099-03-01T13:00:00.000Z', 11)
		].forEach((s) => db.set(s.id, s));
		const set = (id: number, patch: Record<string, unknown>) => {
			const s = { ...db.get(id)!, ...patch };
			db.set(id, s);
			return s;
		};
		const handlers: Record<string, (a: Record<string, unknown>) => unknown> = {
			schedule_list: () => [...db.values()],
			schedule_pause: (a) => set(a.id as number, { status: 'paused' }),
			schedule_reactivate: (a) => set(a.id as number, { status: 'pending' }),
			schedule_cancel: (a) => set(a.id as number, { status: 'cancelled' }),
			schedule_reschedule: (a) => set(a.id as number, { run_at: a.runAt })
		};
		(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
			invoke: async (cmd: string, args: Record<string, unknown>) => {
				const h = handlers[cmd];
				if (!h) throw new Error(`comando não mockado: ${cmd}`);
				return h(args ?? {});
			},
			transformCallback: () => 0,
			metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main', windowLabel: 'main' } }
		};
	});
}

async function open(page: Page) {
	await mockIpc(page);
	await page.emulateMedia({ reducedMotion: 'reduce' });
	for (let i = 0; i < 4; i++) {
		await page.goto('/agenda');
		if (await page.locator('h1').waitFor({ timeout: 20_000 }).then(() => true, () => false)) break;
	}
	await expect(page.locator('h1')).toHaveText('Agenda');
	await expect(page.getByRole('list', { name: 'Agendamentos' })).toBeVisible();
}

const axe = semViolacoesAxe;

test('/agenda sem violações axe (lista e diálogos)', async ({ page }) => {
	await open(page);
	await axe(page);
	await page.getByRole('button', { name: /^Reagendar: Renovação/ }).click();
	await expect(page.getByRole('dialog', { name: 'Reagendar' })).toBeVisible();
	await axe(page);
	await page.keyboard.press('Escape');
	await page.getByRole('button', { name: /^Cancelar renovação agendada:/ }).click();
	await expect(page.getByRole('dialog', { name: 'Cancelar renovação agendada?' })).toBeVisible();
	await axe(page);
});

async function tabTo(page: Page, name: RegExp | string) {
	const target = page.getByRole('button', { name });
	for (let i = 0; i < 40; i++) {
		if (await target.evaluate((el) => el === document.activeElement).catch(() => false)) return;
		await page.keyboard.press('Tab');
	}
	throw new Error(`não alcançou ${name} por Tab`);
}

test('fluxo completo só por teclado', async ({ page }) => {
	await open(page);

	// Pausar e reativar.
	await tabTo(page, /^Pausar: Renovação/);
	await page.keyboard.press('Enter');
	await expect(page.getByRole('status').filter({ hasText: 'Agendamento pausado.' })).toBeVisible();
	await tabTo(page, /^Reativar: Renovação/);
	await page.keyboard.press('Enter');
	await expect(page.getByRole('status').filter({ hasText: 'Agendamento reativado.' })).toBeVisible();

	// Reagendar: fora da janela é recusado, dentro é aceito.
	await tabTo(page, /^Reagendar: Renovação/);
	await page.keyboard.press('Enter');
	const field = page.getByLabel('Nova data e hora (horário de Brasília)');
	await expect(page.getByRole('dialog', { name: 'Reagendar' })).toBeVisible();
	for (let i = 0; i < 10 && !(await field.evaluate((el) => el === document.activeElement)); i++) await page.keyboard.press('Tab');
	await expect(field).toBeFocused();
	await field.fill('2099-03-12T22:00');
	await page.keyboard.press('Enter');
	await expect(page.getByText(/Fora da janela permitida/)).toBeVisible();
	await field.fill('2099-03-12T08:30');
	await page.keyboard.press('Enter');
	await expect(page.getByRole('dialog')).toBeHidden();
	await expect(page.getByRole('heading', { name: 'Renovação · 12/03/2099, 08:30' })).toBeVisible();

	// Cancelar a renovação agendada, com confirmação.
	await tabTo(page, /^Cancelar renovação agendada:/);
	await page.keyboard.press('Enter');
	const dlg = page.getByRole('dialog', { name: 'Cancelar renovação agendada?' });
	await expect(dlg).toBeVisible();
	await tabTo(page, 'Sim, cancelar');
	await page.keyboard.press('Enter');
	await expect(page.getByRole('status').filter({ hasText: 'Renovação agendada cancelada.' })).toBeVisible();
	await expect(page.getByRole('button', { name: /^Reativar: Renovação/ })).toBeVisible();

	// Filtro por teclado.
	const filter = page.getByLabel('Mostrar');
	await filter.focus();
	await filter.selectOption('cancelled');
	await expect(page.getByRole('list', { name: 'Agendamentos' }).getByRole('listitem')).toHaveCount(1);
});
