import type { RequestLogEntry_Serialize } from '#lib/ipc/bindings.ts';

export type DiagnosticRow = {
	id: number;
	at: string;
	host: string;
	kind: string;
	status: string;
	outcome: string;
	account: string;
};

const dateFmt = new Intl.DateTimeFormat('pt-BR', { dateStyle: 'short', timeStyle: 'medium' });

export function formatAt(iso: string): string {
	const d = new Date(iso);
	return Number.isNaN(d.getTime()) ? iso : dateFmt.format(d);
}

/** Converte entradas do log em linhas exibíveis. Só copia campos sem credenciais. */
export function toRows(entries: RequestLogEntry_Serialize[]): DiagnosticRow[] {
	return entries.map((e) => ({
		id: e.id,
		at: formatAt(e.at),
		host: e.host,
		kind: e.kind,
		status: e.http_status == null ? '—' : String(e.http_status),
		outcome: e.outcome,
		account: e.account_id == null ? '—' : `#${e.account_id}`
	}));
}

export const DIAGNOSTICS_LIMIT = 50;
