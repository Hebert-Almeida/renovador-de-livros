// @ts-nocheck -- script Node de manutenção, fora do código do app.
// Gera THIRD-PARTY-NOTICES.md (textos completos das licenças de terceiros que
// vão nos instaladores) e src/lib/features/sobre/licenses.generated.json.
//
// Fontes:
// - Rust: `cargo about` (about.toml), só dependências de execução, offline,
//   lendo os arquivos de licença dos próprios crates;
// - código C compilado no binário por dependências de build (OpenSSL e
//   SQLCipher/SQLite/LibTomCrypt), lido dos fontes vendorizados;
// - JavaScript: pacotes de execução do package-lock.json, com o arquivo de
//   licença de cada pacote em node_modules; o GSAP não traz o texto no pacote,
//   então vale a cópia oficial em docs/legal/licenca-gsap.txt.
//
// Uso:
//   node scripts/gen-notices.mjs           regrava os dois arquivos
//   node scripts/gen-notices.mjs --check   falha se algum estiver desatualizado
// Requer `cargo install cargo-about --locked --features cli` e `npm ci`.
import { execFileSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { licensesJson } from './gen-licenses.mjs';

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

function cargoAbout() {
	const out = execFileSync(
		'cargo',
		['about', 'generate', '--format', 'json', '-c', 'about.toml', '-m', 'src-tauri/Cargo.toml', '--locked', '--offline', '--fail'],
		{ cwd: root, maxBuffer: 512 * 1024 * 1024, stdio: ['ignore', 'pipe', 'inherit'] }
	);
	return JSON.parse(out.toString());
}

function rustSection(about) {
	// Um bloco por texto de licença distinto, com os crates que o usam.
	const blocks = about.licenses
		.map((l) => ({
			name: l.name,
			id: l.id,
			text: norm(l.text).trim(),
			users: [...new Set(l.used_by.map((u) => `${u.crate.name} ${u.crate.version}`))].sort()
		}))
		.sort((a, b) => a.id.localeCompare(b.id) || a.users[0].localeCompare(b.users[0]) || a.text.localeCompare(b.text));
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

function cargoPackage(name) {
	const meta = JSON.parse(
		execFileSync('cargo', ['metadata', '--format-version', '1', '--locked'], {
			cwd: root,
			maxBuffer: 256 * 1024 * 1024
		}).toString()
	);
	const pkg = meta.packages.find((p) => p.name === name);
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

function jsPackages() {
	const lock = JSON.parse(readFileSync(join(root, 'package-lock.json'), 'utf8'));
	return Object.entries(lock.packages)
		.filter(([path, p]) => path.startsWith('node_modules/') && !p.dev && !p.devOptional)
		.map(([path, p]) => ({ path, name: path.slice(path.lastIndexOf('node_modules/') + 13), ...p }))
		.sort((a, b) => a.name.localeCompare(b.name));
}

function jsSection() {
	const parts = [`## 1. Interface (JavaScript)\n`];
	const pkgs = jsPackages();
	parts.push(`${pkgs.length} pacotes de execução incluídos na interface.\n`);
	for (const p of pkgs) {
		const dir = join(root, p.path);
		const meta = JSON.parse(readFileSync(join(dir, 'package.json'), 'utf8'));
		const license = p.license ?? meta.license ?? 'desconhecida';
		const files = readdirSync(dir)
			.filter((f) => /^(licen[cs]e|copying|notice)(?!\.spdx$)/i.test(f))
			.sort();
		let body;
		if (p.name === 'gsap') {
			body = fence(readFileSync(join(root, GSAP_LICENSE), 'utf8'));
		} else if (files.length) {
			body = files.map((f) => (files.length > 1 ? `\`${f}\`:\n\n` : '') + fence(readFileSync(join(dir, f), 'utf8'))).join('\n\n');
		} else {
			const repo = typeof meta.repository === 'string' ? meta.repository : meta.repository?.url;
			body =
				`O pacote não traz o texto da licença. Licença declarada: ${license}` +
				(repo ? `; texto no repositório oficial: ${repo.replace(/^git\+/, '')}` : '') +
				'.';
		}
		parts.push(`### ${p.name} ${p.version} (${license})\n\n${body}\n`);
	}
	return parts.join('\n');
}

export function notices() {
	const about = cargoAbout();
	return [
		`# Avisos de terceiros\n`,
		`<!-- Gerado por \`node scripts/gen-notices.mjs\`. Não edite à mão. -->\n`,
		`O Renovador de Livros é licenciado sob MIT OR Apache-2.0. Ele inclui os componentes de terceiros abaixo, ` +
			`cada um sob a própria licença, reproduzida na íntegra (ou indicada pelo endereço oficial, quando o ` +
			`pacote não traz o texto).\n`,
		jsSection(),
		nativeSection(),
		rustSection(about)
	]
		.join('\n')
		.replace(/\n{3,}/g, '\n\n')
		.trimEnd()
		.concat('\n');
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
	const check = process.argv.includes('--check');
	const outputs = [
		{ path: NOTICES, text: notices() },
		(({ path, text }) => ({ path, text }))(licensesJson())
	];
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
