// Backend de demonstração para as capturas do manual do usuário.
// Dados fictícios (nenhum acesso à rede); autocontido porque é serializado
// para o navegador pelo Playwright (`page.addInitScript`).
export function installDemoBackend(onboarded: boolean) {
	const NOW = '2026-11-17T13:00:00Z'; // 10:00 em Brasília
	const day = (d: number) => new Date(Date.parse('2026-11-17T12:00:00Z') + d * 86_400_000).toISOString().slice(0, 10);
	const at = (d: number, hhmm: string) => {
		const [h, m] = hhmm.split(':').map(Number);
		return new Date(Date.parse(`${day(d)}T00:00:00-03:00`) + (h * 60 + m) * 60_000).toISOString();
	};
	const account = {
		id: 1,
		library_base_url: 'https://biblioteca.sophia.com.br/5362',
		library_name: 'Biblioteca UFTM',
		login: '2022000123',
		has_password: true,
		mode: 'auto',
		status: 'active',
		renew_days_before: 1,
		remind_days_before: 2,
		last_sync_at: at(0, '07:42'),
		last_renewal_at: at(-3, '15:17'),
		next_renewal_at: at(1, '09:23'),
		created_at: at(-120, '18:00'),
		updated_at: at(-3, '15:17')
	};
	const books = [
		['Vidas Secas', 'Graciliano Ramos', '869.93 R175v'],
		['Dom Casmurro', 'Machado de Assis', '869.93 A848d'],
		['O Cortiço', 'Aluísio Azevedo', '869.93 A994c'],
		['Grande Sertão: Veredas', 'João Guimarães Rosa', '869.93 R788g'],
		['Memórias Póstumas de Brás Cubas', 'Machado de Assis', '869.93 A848m']
	].map(([title, author, call], i) => ({
		id: i + 1,
		account_id: 1,
		item_key: `ITEM-${1000 + i}`,
		title,
		author,
		call_number: call,
		library_branch: 'Biblioteca Universitária',
		first_seen_at: at(-60 + i, '12:00')
	}));
	const loanRows: [number, number, number, number | null, string, boolean][] = [
		// book, devolução (dias a partir de hoje), renovações, limite, situação, automática
		[1, 2, 1, 15, 'active', true],
		[2, 5, 3, 15, 'active', true],
		[3, 9, 0, 15, 'active', false],
		[4, -1, 15, 15, 'overdue', false],
		[5, -20, 4, 15, 'returned', false]
	];
	const loans = loanRows.map(([book, due, count, limit, status, auto], i) => ({
		id: i + 1,
		book_id: book,
		loaned_at: at(-30 - i * 4, '14:10'),
		due_date: day(due),
		renewals_count: count,
		renewals_limit: limit,
		status,
		auto_renew: auto,
		returned_at: status === 'returned' ? at(-21, '16:40') : null,
		updated_at: at(-1, '08:00')
	}));
	const view = (l: (typeof loans)[number]) => ({ loan: l, book: books[l.book_id - 1] });
	let eventId = 1;
	const ev = (loan: number, d: number, hhmm: string, result: string, trigger: string, oldDue: number | null, newDue: number | null, message: string | null = null) => ({
		id: eventId++,
		loan_id: loan,
		attempted_at: at(d, hhmm),
		trigger,
		result,
		old_due_date: oldDue == null ? null : day(oldDue),
		new_due_date: newDue == null ? null : day(newDue),
		message
	});
	const events = [
		// Dom Casmurro: falha de rede, recuperação no dia seguinte e duas renovações automáticas.
		ev(2, -3, '15:17', 'success', 'auto', -2, 5),
		ev(2, -10, '11:05', 'success', 'auto', -9, -2),
		ev(2, -17, '08:02', 'success', 'catch_up', -16, -9),
		ev(2, -18, '16:30', 'network_error', 'auto', -16, null, 'Sem resposta da biblioteca.'),
		ev(1, -5, '09:41', 'success', 'manual', -4, 2),
		ev(4, -2, '10:12', 'limit_reached', 'auto', -1, null, 'Limite de renovações atingido.'),
		ev(5, -35, '13:20', 'success', 'auto', -34, -20),
		ev(3, -40, '17:45', 'reserved', 'auto', -39, null, 'O livro tem reserva.')
	].sort((a, b) => (a.attempted_at < b.attempted_at ? 1 : -1));
	let scheduleId = 1;
	const sch = (loan: number | null, kind: string, d: number, hhmm: string, status = 'pending', origin = 'system') => ({
		id: scheduleId++,
		account_id: 1,
		loan_id: loan,
		kind,
		run_at: at(d, hhmm),
		status,
		origin,
		updated_at: NOW
	});
	const schedules = [
		sch(1, 'renew', 1, '09:23'),
		sch(2, 'renew', 4, '09:47'),
		sch(3, 'remind', 7, '09:00'),
		sch(3, 'remind', 9, '09:00'),
		sch(null, 'sync', 3, '08:15'),
		sch(4, 'remind', 0, '09:00', 'done'),
		sch(2, 'renew', 11, '10:30', 'paused', 'user')
	];
	const settings = {
		theme: 'light',
		motion: 'reduced',
		notifications: true,
		autostart: true,
		app_lock: false,
		onboarding_done: onboarded,
		privacy_accepted_at: onboarded ? at(-120, '18:00') : null,
		renew_period: 'morning',
		automation_paused: false
	};
	const quota = {
		account_id: 1,
		manual_sessions_left: 3,
		auto_sessions_left: 2,
		install_sessions_left: 9,
		next_allowed_at: null,
		breaker_open_until: null,
		reason: null
	};
	const monthly = [
		['2026-06', 3, 3],
		['2026-07', 2, 1],
		['2026-08', 5, 4],
		['2026-09', 4, 4],
		['2026-10', 6, 5],
		['2026-11', 3, 3]
	].map(([month, renewals, successes]) => ({
		month,
		renewals,
		successes,
		failures: (renewals as number) - (successes as number)
	}));
	const requests = [
		['login', 200, 'ok', '07:42'],
		['list', 200, 'ok', '07:42'],
		['logout', 200, 'ok', '07:43']
	].map(([kind, status, outcome, hhmm], i) => ({
		id: 30 - i,
		account_id: 1,
		host: 'biblioteca.sophia.com.br',
		at: at(0, hhmm as string),
		kind,
		session_id: 's-0042',
		http_status: status,
		outcome
	}));

	const handlers: Record<string, (a: Record<string, unknown>) => unknown> = {
		app_status: () => ({
			version: '2.0.0-alpha.0',
			accounts: onboarded ? 1 : 0,
			automation: { enabled: true, remote_enabled: true, paused_reason: null },
			next_run_at: at(1, '09:23'),
			onboarding_done: onboarded
		}),
		library_presets: () => [
			{ name: 'Biblioteca UFTM', base_url: 'https://biblioteca.sophia.com.br/5362', supported: true }
		],
		library_validate: (a) => ({ ok: true, normalized_url: a.url, reason: null }),
		account_list: () => (onboarded ? [account] : []),
		quota_status: () => quota,
		loan_list: (a) => {
			const f = (a.filter ?? {}) as { status?: string | null };
			return loans.filter((l) => !f.status || l.status === f.status).map(view);
		},
		loan_get: (a) => {
			const l = loans.find((x) => x.id === a.id)!;
			return {
				view: view(l),
				events: events.filter((e) => e.loan_id === l.id),
				schedules: schedules.filter((s) => s.loan_id === l.id)
			};
		},
		schedule_list: (a) => {
			const f = (a.filter ?? {}) as { status?: string | null; kind?: string | null };
			return schedules.filter((s) => (!f.status || s.status === f.status) && (!f.kind || s.kind === f.kind));
		},
		history_list: (a) => {
			const p = (a.page ?? { offset: 0, limit: 20 }) as { offset: number; limit: number };
			return { items: events.slice(p.offset, p.offset + p.limit), total: events.length };
		},
		stats_overview: () => ({
			distinct_books: 5,
			active_loans: 3,
			renewals_total: 23,
			success_rate: 20 / 23,
			days_gained: 140,
			most_renewed: books[1]
		}),
		stats_monthly: () => monthly,
		settings_get: () => settings,
		settings_set: (a) => Object.assign(settings, a.patch),
		diagnostics_requests: () => requests,
		'plugin:event|listen': (a) => a.handler,
		'plugin:event|unlisten': () => null,
		'plugin:app|version': () => '2.0.0-alpha.0',
		'plugin:autostart|is_enabled': () => true,
		'plugin:notification|is_permission_granted': () => true,
		'plugin:os|platform': () => 'windows'
	};
	let next = 1;
	const w = window as unknown as Record<string, unknown>;
	w.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
	w.__TAURI_OS_PLUGIN_INTERNALS__ = { platform: 'windows', os_type: 'windows', version: '10', arch: 'x86_64', family: 'windows', eol: '\r\n', exe_extension: 'exe' };
	w.__TAURI_INTERNALS__ = {
		metadata: { currentWindow: { label: 'main' }, currentWebview: { windowLabel: 'main', label: 'main' } },
		invoke: async (cmd: string, args: Record<string, unknown>) => {
			const h = handlers[cmd];
			if (!h) {
				console.warn(`demo: comando sem dados: ${cmd}`);
				return null;
			}
			return h(args ?? {});
		},
		transformCallback: () => next++,
		unregisterCallback: () => {},
		convertFileSrc: (p: string) => p
	};
}
