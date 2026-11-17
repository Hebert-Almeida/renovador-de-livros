// @ts-nocheck -- script Node de manutenção, fora do código do app.
// Gera src/lib/features/sobre/licenses.generated.json a partir dos lockfiles.
// JS: dependências de execução do package.json, versão e licença do package-lock.json.
// Rust: fechamento transitivo das dependências normais do app (cargo metadata),
// só nos alvos distribuídos (Windows, Linux, macOS e Android; os mesmos do about.toml).
// Uso: node scripts/gen-licenses.mjs   (rodar após mudar dependências)
import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const out = join(root, 'src/lib/features/sobre/licenses.generated.json');

export function jsLicenses(pkg, lock) {
	return Object.keys(pkg.dependencies ?? {})
		.sort()
		.map((name) => {
			const entry = lock.packages[`node_modules/${name}`];
			if (!entry) throw new Error(`${name} ausente do package-lock.json`);
			return {
				name,
				version: entry.version,
				license: entry.license ?? 'desconhecida'
			};
		});
}

export const TARGETS = [
	'x86_64-pc-windows-msvc',
	'x86_64-unknown-linux-gnu',
	'aarch64-apple-darwin',
	'x86_64-apple-darwin',
	'aarch64-linux-android',
	'armv7-linux-androideabi',
	'i686-linux-android',
	'x86_64-linux-android'
];

/** Ordem por código de caractere: igual em qualquer máquina (localeCompare depende do ICU/locale). */
export const cmp = (a, b) => (a < b ? -1 : a > b ? 1 : 0);

const closures = new Map();

/** Pacotes Rust de execução do app num alvo (memorizado: cada `cargo metadata` leva segundos). */
export function rustFor(target) {
	if (!closures.has(target)) closures.set(target, computeRustFor(target));
	return closures.get(target);
}

function computeRustFor(target) {
	const meta = JSON.parse(
		execFileSync('cargo', ['metadata', '--format-version', '1', '--locked', '--filter-platform', target], {
			cwd: root,
			maxBuffer: 256 * 1024 * 1024
		}).toString()
	);
	const pkgs = new Map(meta.packages.map((p) => [p.id, p]));
	const nodes = new Map(meta.resolve.nodes.map((n) => [n.id, n]));
	const workspace = new Set(meta.workspace_members);
	const app = meta.packages.find((p) => p.name === 'renovador' && workspace.has(p.id));
	if (!app) throw new Error('pacote renovador não encontrado');
	const seen = new Set();
	const stack = [app.id];
	while (stack.length) {
		const id = stack.pop();
		if (seen.has(id)) continue;
		seen.add(id);
		for (const dep of nodes.get(id)?.deps ?? []) {
			// Só dependências de execução (kind null); build e dev não vão no binário.
			if (dep.dep_kinds.some((k) => k.kind === null)) stack.push(dep.pkg);
		}
	}
	return [...seen].filter((id) => !workspace.has(id)).map((id) => pkgs.get(id));
}

export function rustLicenses() {
	const all = new Map();
	for (const target of TARGETS) {
		for (const p of rustFor(target)) all.set(`${p.name}@${p.version}`, p);
	}
	return [...all.values()]
		.map((p) => ({
			name: p.name,
			version: p.version,
			license: p.license ?? 'desconhecida'
		}))
		.sort((a, b) => cmp(a.name, b.name) || cmp(a.version, b.version));
}

/** Conteúdo de licenses.generated.json (tela Sobre). */
export function licensesJson() {
	const pkg = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'));
	const lock = JSON.parse(readFileSync(join(root, 'package-lock.json'), 'utf8'));
	const data = { js: jsLicenses(pkg, lock), rust: rustLicenses() };
	return { path: out, text: JSON.stringify(data, null, 2) + '\n', data };
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
	const { text, data } = licensesJson();
	writeFileSync(out, text);
	console.log(`licenças: ${data.js.length} JS, ${data.rust.length} Rust`);
}
