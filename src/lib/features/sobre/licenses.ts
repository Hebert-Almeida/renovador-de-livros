// Licenças das dependências JavaScript de execução (package.json › dependencies).
// Não há gerador automático no projeto; manter em sincronia com package-lock.json.
export type ThirdPartyLicense = {
	name: string;
	version: string;
	license: string;
};

export const thirdPartyLicenses: ThirdPartyLicense[] = [
	{ name: '@tauri-apps/api', version: '2.12.4', license: 'Apache-2.0 OR MIT' },
	{ name: '@tauri-apps/plugin-autostart', version: '2.7.0', license: 'MIT OR Apache-2.0' },
	{ name: '@tauri-apps/plugin-dialog', version: '2.8.4', license: 'MIT OR Apache-2.0' },
	{ name: '@tauri-apps/plugin-fs', version: '2.7.1', license: 'MIT OR Apache-2.0' },
	{ name: '@tauri-apps/plugin-log', version: '2.10.2', license: 'MIT OR Apache-2.0' },
	{ name: '@tauri-apps/plugin-notification', version: '2.8.1', license: 'MIT OR Apache-2.0' },
	{ name: '@tauri-apps/plugin-opener', version: '2.7.3', license: 'MIT OR Apache-2.0' },
	{ name: '@tauri-apps/plugin-os', version: '2.4.0', license: 'MIT OR Apache-2.0' },
	{ name: 'animejs', version: '4.5.0', license: 'MIT' },
	{ name: 'gsap', version: '3.15.1', license: 'GSAP Standard "No Charge" License' },
	{ name: 'svelte', version: '5.57.3', license: 'MIT' },
	{ name: 'three', version: '0.187.0', license: 'MIT' }
];
