import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { account, appStatus, loans, quota } from './fixtures.ts';

interface Fixture {
	status: unknown;
	accounts: unknown[];
	loans: unknown[];
	quota: unknown;
}

// Mock do IPC do Tauri injetado antes do app (mesma forma do `@tauri-apps/api/mocks`, com eventos).
function installMock(fx: Fixture) {
	const callbacks = new Map<number, (d: unknown) => void>();
	const listeners = new Map<string, number[]>();
	let next = 1;
	const fire = (event: string, payload: unknown) =>
		(listeners.get(event) ?? []).forEach((id) => callbacks.get(id)?.({ event, id, payload }));
	const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
	const handlers: Record<string, (a: Record<string, unknown>) => unknown> = {
		app_status: () => fx.status,
		account_list: () => fx.accounts,
		loan_list: () => fx.loans,
		quota_status: () => fx.quota,
		cycle_run_now: async () => {
			for (const [step, message] of [
				['login', 'Entrando na biblioteca'],
				['list', 'Lendo empréstimos'],
				['save', 'Salvando']
			]) {
				fire('cycle-progress-event', { account_id: 1, step, message, current: null, total: null });
				await sleep(150);
			}
			return { account_id: 1, requests_made: 3, loans_seen: 2, renewed: [], failed: [], returned_detected: 0, error: null };
		},
		'plugin:event|listen': (a) => {
			const ev = a.event as string;
			listeners.set(ev, [...(listeners.get(ev) ?? []), a.handler as number]);
			return a.handler;
		},
		'plugin:event|unlisten': () => null
	};
	const w = window as unknown as Record<string, unknown>;
	w.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
	w.__TAURI_INTERNALS__ = {
		invoke: async (cmd: string, args: Record<string, unknown>) => {
			const h = handlers[cmd];
			if (!h) throw new Error(`comando não mockado: ${cmd}`);
			return h(args ?? {});
		},
		transformCallback: (cb: (d: unknown) => void) => {
			const id = next++;
			callbacks.set(id, cb);
			return id;
		},
		unregisterCallback: (id: number) => callbacks.delete(id),
		convertFileSrc: (p: string) => p
	};
}

const full: Fixture = { status: appStatus(), accounts: [account()], loans: loans(), quota: quota() };

async function open(page: Page, fx: Fixture, tema?: string) {
	await page.emulateMedia({ reducedMotion: 'reduce' });
	await page.addInitScript(installMock, fx);
	if (tema) await page.addInitScript((t) => (document.documentElement.dataset.theme = t), tema);
	// Na primeira carga o Vite (dev) pode reotimizar dependências e recarregar; tenta de novo.
	for (let i = 0; i < 4; i++) {
		await page.goto('/');
		if (await page.locator('h1').waitFor({ timeout: 20_000 }).then(() => true, () => false)) break;
	}
	await expect(page.locator('h1')).toHaveText('Painel');
}

for (const tema of ['light', 'dark', 'contrast']) {
	test(`painel no tema ${tema} sem violações axe`, async ({ page }) => {
		await open(page, full, tema);
		await expect(page.getByRole('heading', { name: 'Próximos vencimentos' })).toBeVisible();
		expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
	});
}

test('estado vazio sem violações axe', async ({ page }) => {
	await open(page, { ...full, accounts: [], loans: [] });
	await expect(page.getByRole('heading', { name: 'Comece cadastrando sua conta' })).toBeVisible();
	expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
});

test('teclado: sincronizar agora e acompanhar o progresso', async ({ page }) => {
	await open(page, full);
	await expect(page.getByText('Restam 2 sincronizações manuais hoje.')).toBeVisible();
	const btn = page.getByRole('button', { name: 'Sincronizar agora' });
	// Chega ao botão só com Tab.
	for (let i = 0; i < 40 && !(await btn.evaluate((el) => el === document.activeElement)); i++) await page.keyboard.press('Tab');
	await expect(btn).toBeFocused();
	await page.keyboard.press('Enter');
	const live = page.locator('.painel__live');
	await expect(live).toHaveText(/Entrando na biblioteca|Lendo empréstimos|Salvando/);
	await expect(live).toHaveText('Sincronização concluída: 2 empréstimos, 0 renovados.');
	expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
	await expect(page.getByRole('button', { name: 'Sincronizar agora' })).toBeFocused();
});

test('teclado: abre um empréstimo pela lista', async ({ page }) => {
	await open(page, full);
	const link = page.getByRole('link', { name: 'Vidas Secas' });
	for (let i = 0; i < 40 && !(await link.evaluate((el) => el === document.activeElement)); i++) await page.keyboard.press('Tab');
	await expect(link).toBeFocused();
	await page.keyboard.press('Enter');
	await expect(page).toHaveURL(/\/emprestimos\/2$/);
});
