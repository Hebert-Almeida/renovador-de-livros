// Licenças de terceiros, geradas dos lockfiles por scripts/gen-licenses.mjs.
// Rodar `node scripts/gen-licenses.mjs` sempre que as dependências mudarem;
// tests/ui/sobre confere a parte JS contra o package-lock.json.
import data from './licenses.generated.json';

export type ThirdPartyLicense = {
	/** `nome@versão`: o Rust pode ter duas versões do mesmo crate. */
	id: string;
	name: string;
	version: string;
	license: string;
};

type Raw = { name: string; version: string; license: string };
const withId = (rows: Raw[]): ThirdPartyLicense[] => rows.map((r) => ({ id: `${r.name}@${r.version}`, ...r }));

/** Dependências JavaScript de execução (package.json › dependencies). */
export const jsLicenses: ThirdPartyLicense[] = withId(data.js);
/** Crates Rust que vão no binário (Windows e Android). */
export const rustLicenses: ThirdPartyLicense[] = withId(data.rust);
