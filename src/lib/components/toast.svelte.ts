// Fila de toasts consumida por <ToastRegion />.
export type ToastKind = 'info' | 'ok' | 'warn' | 'danger';
export interface ToastItem {
	id: number;
	message: string;
	kind: ToastKind;
}

let nextId = 1;
export const toasts: ToastItem[] = $state([]);

export function dismissToast(id: number): void {
	const i = toasts.findIndex((t) => t.id === id);
	if (i >= 0) toasts.splice(i, 1);
}

/** Mostra um toast. `timeout` em ms (0 = permanece até fechar). */
export function toast(message: string, kind: ToastKind = 'info', timeout = 5000): number {
	const id = nextId++;
	toasts.push({ id, message, kind });
	if (timeout > 0) setTimeout(() => dismissToast(id), timeout);
	return id;
}
