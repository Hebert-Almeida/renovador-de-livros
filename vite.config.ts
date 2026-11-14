import { sveltekit } from '@sveltejs/kit/vite';
import adapter from '@sveltejs/adapter-static';
import { defineConfig } from 'vitest/config';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
	plugins: [
		sveltekit({
			adapter: adapter({ fallback: 'index.html' })
		})
	],
	clearScreen: false,
	server: {
		port: 1420,
		strictPort: true,
		host: host || false,
		hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
		watch: { ignored: ['**/src-tauri/**', '**/target/**'] }
	},
	test: {
		environment: 'jsdom',
		include: ['src/**/*.test.ts', 'tests/ui/**/*.test.ts'],
		setupFiles: ['tests/ui/setup.ts']
	},
	resolve: process.env.VITEST ? { conditions: ['browser'] } : {}
});
