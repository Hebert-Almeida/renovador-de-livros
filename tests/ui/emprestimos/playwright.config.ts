// Playwright das telas de empréstimos (IPC mockado no navegador).
// Rode com: npx playwright test -c tests/ui/emprestimos/playwright.config.ts
import { defineConfig } from '@playwright/test';

export default defineConfig({
	testDir: '.',
	testMatch: '*.spec.ts',
	workers: 1,
	timeout: 120_000,
	expect: { timeout: 30_000 },
	webServer: { command: 'npx vite dev --port 5206', cwd: '../../..', port: 5206, reuseExistingServer: false, timeout: 120_000 },
	use: { baseURL: 'http://localhost:5206' }
});
