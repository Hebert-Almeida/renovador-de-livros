// @ts-nocheck -- script Node de manutenção, fora do código do app.
// Gera THIRD-PARTY-NOTICES.md (textos completos das licenças de terceiros que
// vão nos instaladores) e src/lib/features/sobre/licenses.generated.json.
//
// Fontes:
// - JavaScript: os pacotes que o Vite de fato coloca no bundle do cliente
//   (inclui o runtime do Svelte/SvelteKit, que são devDependencies) mais as
//   dependências de execução do package.json; texto do arquivo de licença de
//   cada pacote. O GSAP não traz o texto no pacote: vale a cópia oficial em
//   docs/legal/licenca-gsap.txt;
// - código C compilado no binário por dependências de build (OpenSSL e
//   SQLCipher/SQLite/LibTomCrypt), lido dos fontes vendorizados;
// - Android: dependências Maven declaradas nos módulos Gradle que entram no APK;
// - Rust: `cargo about` (about.toml), só dependências de execução, offline,
//   lendo os arquivos de licença dos próprios crates.
//
// Uso:
//   node scripts/gen-notices.mjs           regrava os dois arquivos
//   node scripts/gen-notices.mjs --check   falha se algum estiver desatualizado
// Requer `cargo install cargo-about --locked --version 0.9.3 --features cli` e `npm ci`.
import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { cmp, licensesJson, rustFor } from './gen-licenses.mjs';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const NOTICES = join(root, 'THIRD-PARTY-NOTICES.md');
const GSAP_LICENSE = 'docs/legal/licenca-gsap.txt';

const norm = (s) => s.replace(/\r\n/g, '\n');
const fence = (text) => {
	const body = norm(text).trim();
	// cerca maior que qualquer sequência de crases do texto
	const ticks = '`'.repeat(Math.max(3, ...[...body.matchAll(/`+/g)].map((m) => m[0].length + 1)));
	return `${ticks}text\n${body}\n${ticks}`;
};

// ---------------------------------------------------------------- JavaScript

/** Pacotes npm presentes no bundle do cliente, medidos num build do Vite. */
function bundledPackages() {
	const dir = mkdtempSync(join(tmpdir(), 'renovador-notices-'));
	const out = join(dir, 'packages.json');
	try {
		execFileSync(process.execPath, [join(root, 'node_modules/vite/bin/vite.js'), 'build', '--logLevel', 'error'], {
			cwd: root,
			env: { ...process.env, RENOVADOR_BUNDLED_PACKAGES: out },
			stdio: ['ignore', 'ignore', 'inherit']
		});
		return JSON.parse(readFileSync(out, 'utf8'));
	} finally {
		rmSync(dir, { recursive: true, force: true });
	}
}

function jsSection() {
	const pkg = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'));
	const lock = JSON.parse(readFileSync(join(root, 'package-lock.json'), 'utf8'));
	const names = [...new Set([...bundledPackages(), ...Object.keys(pkg.dependencies ?? {})])].sort(cmp);
	const parts = [
		`## 1. Interface (JavaScript)\n`,
		`${names.length} pacotes: os que o build coloca na interface e as dependências de execução.\n`
	];
	for (const name of names) {
		const dir = join(root, 'node_modules', name);
		const meta = JSON.parse(readFileSync(join(dir, 'package.json'), 'utf8'));
		const license = lock.packages[`node_modules/${name}`]?.license ?? meta.license ?? 'desconhecida';
		const files = readdirSync(dir)
			.filter((f) => /^(licen[cs]e|copying|notice)(?!\.spdx$)/i.test(f))
			.sort(cmp);
		let body;
		if (name === 'gsap') {
			body = fence(readFileSync(join(root, GSAP_LICENSE), 'utf8'));
		} else if (files.length) {
			body = files
				.map((f) => (files.length > 1 ? `\`${f}\`:\n\n` : '') + fence(readFileSync(join(dir, f), 'utf8')))
				.join('\n\n');
		} else {
			const repo = typeof meta.repository === 'string' ? meta.repository : meta.repository?.url;
			body =
				`O pacote não traz o texto da licença. Licença declarada: ${license}` +
				(repo ? `; texto no repositório oficial: ${repo.replace(/^git\+/, '')}` : '') +
				'.';
		}
		parts.push(`### ${name} ${meta.version} (${license})\n\n${body}\n`);
	}
	return parts.join('\n');
}

// ---------------------------------------------------------------- Rust/C

let metadata;
function cargoPackage(name) {
	metadata ??= JSON.parse(
		execFileSync('cargo', ['metadata', '--format-version', '1', '--locked'], {
			cwd: root,
			maxBuffer: 256 * 1024 * 1024
		}).toString()
	);
	const pkg = metadata.packages.find((p) => p.name === name);
	if (!pkg) throw new Error(`${name} ausente do Cargo.lock`);
	return { version: pkg.version, dir: dirname(pkg.manifest_path) };
}

function nativeSection() {
	const ssl = cargoPackage('openssl-src');
	const sqlc = cargoPackage('sqlcipher-src');
	const read = (dir, rel) => readFileSync(join(dir, rel), 'utf8');
	// versão das bibliotecas embutida no número do crate, ex. 300.6.1+3.6.3
	const sslVersion = ssl.version.split('+')[1] ?? ssl.version;
	const sqlcMeta = Object.fromEntries(
		[...sqlc.version.matchAll(/(sqlcipher|sqlite|libtomcrypt)-([\d.]+)/g)].map((m) => [m[1], m[2]])
	);
	return [
		`## 2. Bibliotecas nativas compiladas no binário\n`,
		`Código C compilado junto com o app (via \`openssl-src\` ${ssl.version} e \`sqlcipher-src\` ${sqlc.version}).\n`,
		`### OpenSSL ${sslVersion} (Apache-2.0)\n\n${fence(read(ssl.dir, 'openssl/LICENSE.txt'))}\n`,
		`### SQLCipher ${sqlcMeta.sqlcipher ?? ''} (BSD-3-Clause)\n\n${fence(read(sqlc.dir, 'sqlcipher/LICENSE-sqlcipher'))}\n`,
		`### SQLite ${sqlcMeta.sqlite ?? ''} (domínio público)\n\n` +
			`O SQLite, base do SQLCipher, está em domínio público: https://sqlite.org/copyright.html\n`,
		`### LibTomCrypt ${sqlcMeta.libtomcrypt ?? ''} (distribuída com o SQLCipher)\n\n${fence(read(sqlc.dir, 'sqlcipher/LICENSE-libtomcrypt'))}\n`
	].join('\n');
}

// ---------------------------------------------------------------- Android

// Licença por grupo Maven (conferida nos POMs publicados). Grupo novo sem
// entrada aqui faz o script falhar, para alguém conferir a licença.
const MAVEN_LICENSES = [
	[/^androidx\./, 'Apache-2.0', 'Google (AndroidX)'],
	[/^com\.google\.android\.material$/, 'Apache-2.0', 'Google (Material Components)'],
	[/^com\.fasterxml\.jackson\./, 'Apache-2.0', 'FasterXML (Jackson)'],
	[/^org\.jetbrains\.kotlin(x)?$/, 'Apache-2.0', 'JetBrains (Kotlin)']
];

function gradleDeps(file) {
	const text = readFileSync(file, 'utf8');
	return [...text.matchAll(/^\s*(?:implementation|api)\("([^:"]+):([^:"]+):([^:"]+)"\)/gm)].map((m) => ({
		group: m[1],
		artifact: m[2],
		version: m[3]
	}));
}

function androidSection() {
	// Módulos Gradle no APK: app, plugin próprio, runtime tauri-android e os
	// plugins Tauri que têm parte Android (crates no fechamento do alvo Android).
	const files = [
		join(root, 'src-tauri/gen/android/app/build.gradle.kts'),
		join(root, 'plugins/tauri-plugin-renovador-bg/android/build.gradle.kts')
	];
	for (const p of rustFor('aarch64-linux-android')) {
		const dir = dirname(p.manifest_path);
		for (const rel of ['android/build.gradle.kts', 'mobile/android/build.gradle.kts']) {
			if (existsSync(join(dir, rel))) files.push(join(dir, rel));
		}
	}
	// O Gradle resolve conflitos pela maior versão.
	const byId = new Map();
	for (const d of files.flatMap(gradleDeps)) {
		const id = `${d.group}:${d.artifact}`;
		const prev = byId.get(id);
		const newer =
			!prev ||
			d.version.localeCompare(prev.version, 'en', { numeric: true, sensitivity: 'base' }) > 0;
		if (newer) byId.set(id, d);
	}
	const rows = [...byId.keys()].sort(cmp).map((id) => {
		const d = byId.get(id);
		const lic = MAVEN_LICENSES.find(([re]) => re.test(d.group));
		if (!lic) throw new Error(`licença desconhecida para ${id}: adicione o grupo em MAVEN_LICENSES`);
		return `| ${id} | ${d.version} | ${lic[1]} | ${lic[2]} |`;
	});
	return [
		`## 4. Android (bibliotecas Java/Kotlin)\n`,
		`Só no APK/AAB. Dependências declaradas nos módulos Gradle do app, do plugin de segundo plano, do runtime ` +
			`\`tauri-android\` e dos plugins do Tauri (o código desses módulos está na seção 3, crates \`tauri\` e ` +
			`\`tauri-plugin-*\`). As dependências transitivas delas (AndroidX, Kotlin) seguem a mesma licença dos ` +
			`respectivos grupos. O texto completo da Apache-2.0 está na seção 3.\n`,
		`| Artefato | Versão | Licença | Autor |\n|---|---|---|---|\n${rows.join('\n')}\n`
	].join('\n');
}

function rustSection(about) {
	// Um bloco por texto de licença distinto, com os crates que o usam.
	const blocks = about.licenses
		.map((l) => ({
			name: l.name,
			id: l.id,
			text: norm(l.text).trim(),
			users: [...new Set(l.used_by.map((u) => `${u.crate.name} ${u.crate.version}`))].sort(cmp)
		}))
		.sort((a, b) => cmp(a.id, b.id) || cmp(a.users[0], b.users[0]) || cmp(a.text, b.text));
	const crates = new Set(about.crates.map((c) => `${c.package.name} ${c.package.version}`));
	const parts = [
		`## 3. Núcleo e aplicativo (Rust)\n`,
		`${crates.size} crates. Quando um crate oferece mais de uma licença à escolha, vale a listada aqui.\n`
	];
	for (const b of blocks) {
		parts.push(`### ${b.name} (${b.id})\n\nUsada por: ${b.users.join(', ')}.\n\n${fence(b.text)}\n`);
	}
	return parts.join('\n');
}

function cargoAbout() {
	// Saída por arquivo: chamado do PowerShell, o cargo-about recusa escrever no stdout.
	const dir = mkdtempSync(join(tmpdir(), 'renovador-about-'));
	const out = join(dir, 'about.json');
	try {
		execFileSync(
			'cargo',
			// prettier-ignore
			['about', 'generate', '--format', 'json', '-c', 'about.toml', '-m', 'src-tauri/Cargo.toml', '--locked', '--offline', '--fail', '-o', out],
			{ cwd: root, stdio: ['ignore', 'ignore', 'inherit'] }
		);
		return JSON.parse(readFileSync(out, 'utf8'));
	} finally {
		rmSync(dir, { recursive: true, force: true });
	}
}

export function notices() {
	return [
		`# Avisos de terceiros\n`,
		`<!-- Gerado por \`node scripts/gen-notices.mjs\`. Não edite à mão. -->\n`,
		`O Renovador de Livros é licenciado sob MIT OR Apache-2.0. Ele inclui os componentes de terceiros abaixo, ` +
			`cada um sob a própria licença, reproduzida na íntegra (ou indicada pelo endereço oficial, quando o ` +
			`pacote não traz o texto).\n`,
		jsSection(),
		nativeSection(),
		rustSection(cargoAbout()),
		androidSection()
	]
		.join('\n')
		.replace(/\n{3,}/g, '\n\n')
		.trimEnd()
		.concat('\n');
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
	const check = process.argv.includes('--check');
	const outputs = [{ path: NOTICES, text: notices() }, licensesJson()];
	let stale = 0;
	for (const { path, text } of outputs) {
		const rel = path.slice(root.length + 1).replace(/\\/g, '/');
		if (check) {
			const current = existsSync(path) ? norm(readFileSync(path, 'utf8')) : '';
			if (current !== text) {
				stale++;
				console.error(`desatualizado: ${rel} (rode node scripts/gen-notices.mjs)`);
			}
		} else {
			writeFileSync(path, text);
			console.log(`gravado: ${rel}`);
		}
	}
	if (stale) process.exit(1);
}
