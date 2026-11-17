// Dados de exemplo do Painel, compartilhados entre Vitest e Playwright (sem dependÃªncias).
import type { Account, AppStatus, LoanView, QuotaStatus } from '#lib/ipc/bindings.ts';

export function appStatus(over: Partial<AppStatus> = {}): AppStatus {
	return {
		version: '2.0.0',
		accounts: 1,
		automation: { enabled: true, remote_enabled: true, paused_reason: null },
		next_run_at: '2026-11-16T09:00:00Z',
		onboarding_done: true,
		...over
	} as AppStatus;
}

export function account(over: Partial<Account> = {}): Account {
	return {
		id: 1,
		library_base_url: 'https://biblioteca.example',
		library_name: 'Biblioteca UFTM',
		login: '2024001',
		has_password: true,
		mode: 'auto',
		status: 'active',
		renew_days_before: 2,
		remind_days_before: 3,
		last_sync_at: '2026-11-14T08:00:00Z',
		last_renewal_at: '2026-11-10T08:05:00Z',
		next_renewal_at: '2026-11-18T09:00:00Z',
		created_at: '2026-01-01T00:00:00Z',
		updated_at: '2026-11-14T08:00:00Z',
		...over
	} as Account;
}

function view(id: number, title: string, author: string, due: string, status: 'active' | 'returned' | 'overdue' = 'active'): LoanView {
	return {
		loan: {
			id,
			book_id: id,
			loaned_at: '2026-10-20',
			due_date: due,
			renewals_count: 1,
			renewals_limit: 5,
			status,
			auto_renew: true,
			returned_at: null,
			updated_at: '2026-11-10T08:05:00Z'
		},
		book: {
			id,
			account_id: 1,
			item_key: `k${id}`,
			title,
			author,
			call_number: null,
			library_branch: null,
			first_seen_at: '2026-10-20T00:00:00Z'
		}
	} as LoanView;
}

export function loans(): LoanView[] {
	return [
		view(1, 'Grande SertÃ£o: Veredas', 'GuimarÃ£es Rosa', '2026-12-02'),
		view(2, 'Vidas Secas', 'Graciliano Ramos', '2026-11-18'),
		view(3, 'Dom Casmurro (devolvido)', 'Machado de Assis', '2026-11-01', 'returned')
	];
}

export function quota(over: Partial<QuotaStatus> = {}): QuotaStatus {
	return {
		account_id: 1,
		manual_sessions_left: 2,
		auto_sessions_left: 4,
		install_sessions_left: 10,
		next_allowed_at: null,
		breaker_open_until: null,
		reason: null,
		...over
	} as QuotaStatus;
}
