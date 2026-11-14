// Helper de testes: mocka o IPC do Tauri.
import { mockIPC, clearMocks } from '@tauri-apps/api/mocks';

export type Handlers = Record<string, (args: Record<string, unknown>) => unknown>;

export function mockCommands(handlers: Handlers): void {
	mockIPC((cmd, args) => {
		const h = handlers[cmd];
		if (!h) throw new Error(`comando não mockado: ${cmd}`);
		return h((args ?? {}) as Record<string, unknown>);
	});
}

export { clearMocks };
