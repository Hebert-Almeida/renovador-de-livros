// @ts-nocheck -- script Node de manutenção, fora do código do app.
// Roda todas as suítes Playwright de tela (IPC mockado, axe e fluxos só por teclado)
// e as de rotas/orçamento, uma de cada vez (cada uma sobe o próprio servidor Vite).
//
// Uso: node scripts/test-telas.mjs [--retries=N]
// Requer `npx playwright install chromium` (no CI: `--with-deps`).
import { spawnSync } from 'node:child_process';
import { readdirSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const cli = join(root, 'node_modules/@playwright/test/cli.js');
const extra = process.argv.slice(2).filter((a) => a.startsWith('--retries='));

const suites = [
	...readdirSync(join(root, 'tests/ui'))
		.map((d) => `tests/ui/${d}/playwright.config.ts`)
		.filter((c) => existsSync(join(root, c)))
		.sort(),
	'playwright.config.ts' // tests/e2e: rotas com axe e orçamento de JS
];

const falhas = [];
for (const config of suites) {
	console.log(`\n=== ${config}`);
	const r = spawnSync(process.execPath, [cli, 'test', '-c', config, '--reporter=line', ...extra], {
		cwd: root,
		stdio: 'inherit'
	});
	if (r.status !== 0) falhas.push(config);
}

console.log(`\n${suites.length - falhas.length}/${suites.length} suítes passaram.`);
if (falhas.length) {
	console.error(`Falharam:\n${falhas.map((f) => `- ${f}`).join('\n')}`);
	process.exit(1);
}
