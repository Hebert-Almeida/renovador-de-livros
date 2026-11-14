// Playwright do kit de componentes: /dev/kit só existe em `vite dev`.
// Rode com: npx playwright test -c tests/ui/components/playwright.config.ts
import { defineConfig } from '@playwright/test';

export default defineConfig({
	testDir: '.',
	testMatch: '*.spec.ts',
	workers: 1,
	timeout: 120_000,
	expect: { timeout: 30_000 },
	webServer: { command: 'npx vite dev --port 5197', cwd: '../../..', port: 5197, reuseExistingServer: false, timeout: 120_000 },
	use: { baseURL: 'http://localhost:5197' }
});
