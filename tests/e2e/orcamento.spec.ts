// Orçamento de performance (PLANO-V2 5.7): JS inicial ≤ 150 KB gzip por rota, com o
// Three.js fora da carga inicial (só sob demanda, nos momentos decorativos).
// Mede o build de produção servido pelo `vite preview` (webServer do playwright.config.ts).
import { test, expect, type Page } from '@playwright/test';
import { gzipSync } from 'node:zlib';

const LIMITE_GZ = 150 * 1024;
const rotas = ['/', '/onboarding', '/emprestimos', '/agenda', '/historico', '/estatisticas', '/contas', '/ajustes', '/sobre'];
/** Rotas com momento decorativo em 3D (o Three é importado sob demanda depois da abertura). */
const COM_THREE = new Set(['/onboarding']);

async function medir(page: Page, rota: string) {
	const scripts = new Map<string, Promise<Buffer>>();
	page.on('response', (res) => {
		const url = res.url();
		if (url.endsWith('.js') && !scripts.has(url)) scripts.set(url, res.body());
	});
	await page.goto(rota);
	await expect(page.locator('h1')).toBeVisible();
	await page.waitForLoadState('networkidle');
	let inicial = 0;
	const three: string[] = [];
	for (const [url, body] of scripts) {
		const buf = await body;
		if (buf.includes('WebGLRenderer')) three.push(url);
		else inicial += gzipSync(buf, { level: 9 }).length;
	}
	return { inicial, three };
}

for (const r of rotas) {
	test(`rota ${r}: JS inicial dentro do orçamento`, async ({ page }) => {
		const { inicial, three } = await medir(page, r);
		const kb = (inicial / 1024).toFixed(1);
		test.info().annotations.push({ type: 'js-inicial-gz', description: `${kb} KB` });
		expect(inicial, `JS inicial de ${r}: ${kb} KB gz`).toBeLessThanOrEqual(LIMITE_GZ);
		if (!COM_THREE.has(r)) expect(three, `Three.js carregado em ${r}`).toEqual([]);
	});

	test(`rota ${r}: com movimento reduzido o Three não é baixado`, async ({ page }) => {
		await page.emulateMedia({ reducedMotion: 'reduce' });
		const { three } = await medir(page, r);
		expect(three).toEqual([]);
	});
}
