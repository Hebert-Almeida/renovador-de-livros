// Playwright da agenda. Rode com: npx playwright test -c tests/ui/agenda/playwright.config.ts
import { defineConfig } from '@playwright/test';

export default defineConfig({
	testDir: '.',
	testMatch: '*.spec.ts',
	workers: 1,
	timeout: 120_000,
	expect: { timeout: 30_000 },
	webServer: { command: 'npx vite dev --port 5207', cwd: '../../..', port: 5207, reuseExistingServer: false, timeout: 120_000 },
	use: { baseURL: 'http://localhost:5207' }
});
