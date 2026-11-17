// Capturas de tela do manual do usuário (docs/manual/img), com dados fictícios.
// Rode com: npx playwright test -c tests/manual/playwright.config.ts
import { defineConfig } from '@playwright/test';

export default defineConfig({
	testDir: '.',
	testMatch: '*.spec.ts',
	workers: 1,
	timeout: 120_000,
	expect: { timeout: 30_000 },
	webServer: { command: 'npx vite dev --port 5240', cwd: '../..', port: 5240, reuseExistingServer: false, timeout: 120_000 },
	use: { baseURL: 'http://localhost:5240', viewport: { width: 1100, height: 760 }, deviceScaleFactor: 1, locale: 'pt-BR', timezoneId: 'America/Sao_Paulo' }
});
