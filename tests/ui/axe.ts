// Verificação axe compartilhada pelas suítes Playwright de tela.
import AxeBuilder from '@axe-core/playwright';
import { expect, type Page } from '@playwright/test';

/**
 * Exige zero violações axe. A troca de tema anima as cores e o Vite (dev) pode
 * recarregar na primeira visita; como o axe mede a cor computada, um quadro
 * intermediário dá falso positivo de contraste. Por isso repete por até 10 s:
 * uma violação real persiste e reprova.
 */
export async function semViolacoesAxe(page: Page) {
	await expect
		.poll(async () => (await new AxeBuilder({ page }).analyze()).violations, { timeout: 10_000 })
		.toEqual([]);
}
