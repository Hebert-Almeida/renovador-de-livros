import { sveltekit } from '@sveltejs/kit/vite';
import adapter from '@sveltejs/adapter-static';
import { defineConfig } from 'vitest/config';
import type { Plugin } from 'vite';
import { writeFileSync } from 'node:fs';

// Com RENOVADOR_BUNDLED_PACKAGES=<arquivo>, grava os pacotes npm que entram no
// bundle do cliente (usado por scripts/gen-notices.mjs para os avisos de terceiros).
function bundledPackages(out: string | undefined): Plugin {
	return {
		name: 'renovador-bundled-packages',
		apply: 'build',
		generateBundle(_options, bundle) {
			if (!out || this.environment?.name !== 'client') return;
			const pkgs = new Set<string>();
			for (const item of Object.values(bundle)) {
				if (item.type !== 'chunk') continue;
				for (const id of item.moduleIds) {
					const m = id.replace(/\\/g, '/').match(/.*\/node_modules\/((?:@[^/]+\/)?[^/]+)\//);
					if (m) pkgs.add(m[1]);
				}
			}
			writeFileSync(out, JSON.stringify([...pkgs].sort(), null, 2) + '\n');
		}
	};
}

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
	plugins: [
		sveltekit({
			adapter: adapter({ fallback: 'index.html' })
		}),
		bundledPackages(process.env.RENOVADOR_BUNDLED_PACKAGES)
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
