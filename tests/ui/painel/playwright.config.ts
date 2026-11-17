// Playwright do Painel (/), com o IPC do Tauri mockado.
// Rode com: npx playwright test -c tests/ui/painel/playwright.config.ts
import { defineConfig } from '@playwright/test';

export default defineConfig({
	testDir: '.',
	testMatch: '*.spec.ts',
	workers: 1,
	timeout: 120_000,
	expect: { timeout: 30_000 },
	webServer: { command: 'npx vite dev --port 5235', cwd: '../../..', port: 5235, reuseExistingServer: false, timeout: 120_000 },
	use: { baseURL: 'http://localhost:5235' }
});
