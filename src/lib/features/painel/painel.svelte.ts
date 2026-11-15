// Estado reativo do Painel: carrega dados pelo IPC tipado e acompanha os ciclos.
import { commands, events } from '#lib/ipc/index.ts';
import type { Account, AppStatus, CycleProgress, CycleReport, LoanView, QuotaStatus } from '#lib/ipc/bindings.ts';
import type { Step, StepState } from '#lib/components/ProgressSteps.svelte';
import t from '#lib/i18n/pt-BR/painel.ts';
import { errorMessage } from './format.ts';

const STEP_ORDER = ['login', 'list', 'renew', 'save'] as const;
type StepId = (typeof STEP_ORDER)[number];

export class PainelState {
	loading = $state(true);
	loadError = $state<string | null>(null);
	status = $state<AppStatus | null>(null);
	accounts = $state<Account[]>([]);
	loans = $state<LoanView[]>([]);
	quotas = $state<Record<number, QuotaStatus>>({});
	selectedAccount = $state<number | null>(null);

	syncing = $state(false);
	progress = $state<CycleProgress | null>(null);
	/** Mensagem anunciada na região `aria-live`. */
	announcement = $state('');

	steps: Step[] = $derived.by(() => {
		const p = this.progress;
		const idx = p ? (p.step === 'confirm' ? 2 : p.step === 'done' ? STEP_ORDER.length : STEP_ORDER.indexOf(p.step as StepId)) : -1;
		return STEP_ORDER.map((id, i): Step => {
			let state: StepState = 'pending';
			if (idx > i) state = 'done';
			else if (idx === i) state = 'active';
			return { id, label: t.steps[id], state };
		});
	});

	/** `quiet` recarrega sem trocar a tela pelo esqueleto (mantém o foco). */
	async load(quiet = false): Promise<void> {
		if (!quiet) this.loading = true;
		this.loadError = null;
		try {
			const [status, accounts, loans] = await Promise.all([
				commands.appStatus(),
				commands.accountList(),
				commands.loanList({ account_id: null, status: null, search: null })
			]);
			this.status = status;
			this.accounts = accounts;
			this.loans = loans
				.filter((l) => l.loan.status !== 'returned')
				.sort((a, b) => a.loan.due_date.localeCompare(b.loan.due_date));
			if (this.selectedAccount === null || !accounts.some((a) => a.id === this.selectedAccount))
				this.selectedAccount = accounts[0]?.id ?? null;
			await this.refreshQuotas();
		} catch (e) {
			this.loadError = errorMessage(e);
		} finally {
			this.loading = false;
		}
	}

	async refreshQuotas(): Promise<void> {
		const entries = await Promise.all(
			this.accounts.map(async (a) => [a.id, await commands.quotaStatus(a.id)] as const)
		);
		this.quotas = Object.fromEntries(entries);
	}

	/** Executa "Sincronizar agora" na conta selecionada. Retorna o relatório ou a mensagem de erro. */
	async sync(): Promise<{ ok: true; report: CycleReport } | { ok: false; message: string }> {
		const id = this.selectedAccount;
		if (id === null || this.syncing) return { ok: false, message: t.syncFailed };
		this.syncing = true;
		this.progress = null;
		this.announcement = t.syncing;
		try {
			const report = await commands.cycleRunNow(id, 'sync');
			if (report.error) throw { kind: 'other', message: report.error };
			this.progress = { account_id: id, step: 'done', message: '', current: null, total: null };
			this.announcement = t.syncDone(report.loans_seen, report.renewed.length);
			return { ok: true, report };
		} catch (e) {
			const message = errorMessage(e);
			this.announcement = message;
			return { ok: false, message };
		} finally {
			this.syncing = false;
			await this.load(true);
		}
	}

	onProgress(p: CycleProgress): void {
		if (this.selectedAccount !== null && p.account_id !== this.selectedAccount) return;
		this.progress = p;
		const count = p.current != null && p.total != null ? ` (${p.current} de ${p.total})` : '';
		if (p.message) this.announcement = p.message + count;
	}

	/** Assina os eventos do backend; devolve a função de limpeza. */
	subscribe(): () => void {
		const offs: Promise<() => void>[] = [
			events.cycleProgressEvent.listen((e) => this.onProgress(e.payload)),
			events.automationStateEvent.listen((e) => {
				if (this.status) this.status = { ...this.status, automation: e.payload } as AppStatus;
			}),
			events.cycleFinishedEvent.listen(() => {
				if (!this.syncing) void this.load(true);
			})
		];
		return () => offs.forEach((p) => p.then((off) => off()).catch(() => {}));
	}
}
