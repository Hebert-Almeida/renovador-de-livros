// Backend falso das contas, autocontido (é serializado para o navegador no Playwright).
export function installFakeBackend(seed: number): (cmd: string, args?: Record<string, unknown>) => unknown {
	type Acc = Record<string, unknown> & { id: number; login: string };
	const now = '2026-11-15T12:00:00Z';
	const accounts: Acc[] = [];
	let nextId = 1;
	let manualLeft = 3;
	const make = (login: string): Acc => ({
		id: nextId++,
		library_base_url: 'https://sophia.uftm.edu.br',
		library_name: 'UFTM',
		login,
		has_password: true,
		mode: 'auto',
		status: 'active',
		renew_days_before: 2,
		remind_days_before: 3,
		last_sync_at: null,
		last_renewal_at: null,
		next_renewal_at: null,
		created_at: now,
		updated_at: now
	});
	for (let i = 0; i < seed; i++) accounts.push(make(`10${i}`));
	const calls: { cmd: string; args?: Record<string, unknown> }[] = [];
	(globalThis as unknown as { __contasCalls: typeof calls }).__contasCalls = calls;

	return (cmd, args) => {
		calls.push({ cmd, args });
		switch (cmd) {
			case 'account_list':
				return accounts.map((a) => ({ ...a }));
			case 'library_presets':
				return [{ name: 'UFTM', base_url: 'https://sophia.uftm.edu.br', supported: true }];
			case 'library_validate':
				return { ok: true, normalized_url: args?.url, reason: null };
			case 'account_create': {
				if (accounts.length >= 3) throw { kind: 'guard', message: { reason: 'account_limit' } };
				const n = args?.account as Record<string, unknown>;
				const a = { ...make(String(n.login)), mode: n.mode, has_password: n.password != null };
				accounts.push(a);
				return a;
			}
			case 'account_update': {
				const a = accounts.find((x) => x.id === args?.id);
				if (!a) throw { kind: 'not_found', message: 'conta' };
				const p = args?.patch as Record<string, unknown>;
				if (p.login) a.login = String(p.login);
				if (p.library_base_url) a.library_base_url = p.library_base_url;
				if (p.mode) a.mode = p.mode;
				if (p.password) a.has_password = true;
				if (p.clear_password) a.has_password = false;
				return { ...a };
			}
			case 'account_delete': {
				const i = accounts.findIndex((x) => x.id === args?.id);
				if (i >= 0) accounts.splice(i, 1);
				return null;
			}
			case 'quota_status':
				return {
					account_id: args?.accountId,
					manual_sessions_left: manualLeft,
					auto_sessions_left: 4,
					install_sessions_left: 10,
					next_allowed_at: null,
					breaker_open_until: null,
					reason: null
				};
			case 'account_test_login':
				manualLeft = Math.max(0, manualLeft - 1);
				return true;
			default:
				throw { kind: 'not_implemented', message: cmd };
		}
	};
}
