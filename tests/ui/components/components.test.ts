import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, fireEvent, cleanup } from '@testing-library/svelte';
import { createRawSnippet, flushSync } from 'svelte';
import Button from '#lib/components/Button.svelte';
import IconButton from '#lib/components/IconButton.svelte';
import Toggle from '#lib/components/Toggle.svelte';
import Tabs from '#lib/components/Tabs.svelte';
import PasswordField from '#lib/components/PasswordField.svelte';
import TextField from '#lib/components/TextField.svelte';
import Select from '#lib/components/Select.svelte';
import DataTable from '#lib/components/DataTable.svelte';
import StatusPill from '#lib/components/StatusPill.svelte';
import ProgressSteps from '#lib/components/ProgressSteps.svelte';
import Stamp from '#lib/components/Stamp.svelte';
import Spine from '#lib/components/Spine.svelte';
import Skeleton from '#lib/components/Skeleton.svelte';
import EmptyState from '#lib/components/EmptyState.svelte';
import Ficha from '#lib/components/Ficha.svelte';
import Dialog from '#lib/components/Dialog.svelte';
import ToastRegion from '#lib/components/ToastRegion.svelte';
import { toast, toasts } from '#lib/components/toast.svelte.ts';

afterEach(() => cleanup());

const text = (t: string) => createRawSnippet(() => ({ render: () => `<span>${t}</span>` }));

describe('Button / IconButton', () => {
	it('dispara onclick e respeita loading', async () => {
		const onclick = vi.fn();
		render(Button, { children: text('Renovar'), onclick });
		await fireEvent.click(screen.getByRole('button', { name: 'Renovar' }));
		expect(onclick).toHaveBeenCalledOnce();
	});
	it('loading desabilita e marca aria-busy', () => {
		render(Button, { children: text('Ok'), loading: true });
		const b = screen.getByRole('button');
		expect(b).toBeDisabled();
		expect(b).toHaveAttribute('aria-busy', 'true');
	});
	it('IconButton tem nome acessível', () => {
		render(IconButton, { label: 'Atualizar', children: text('↻') });
		expect(screen.getByRole('button', { name: 'Atualizar' })).toBeInTheDocument();
	});
});

describe('Toggle', () => {
	it('é um switch que alterna aria-checked', async () => {
		const onchange = vi.fn();
		render(Toggle, { label: 'Automático', onchange });
		const sw = screen.getByRole('switch', { name: 'Automático' });
		expect(sw).toHaveAttribute('aria-checked', 'false');
		await fireEvent.click(sw);
		expect(sw).toHaveAttribute('aria-checked', 'true');
		expect(onchange).toHaveBeenCalledWith(true);
	});
});

describe('Tabs (teclado)', () => {
	it('setas, Home e End movem seleção e foco', async () => {
		render(Tabs, {
			label: 'Visões',
			tabs: [
				{ id: 'a', label: 'A' },
				{ id: 'b', label: 'B' },
				{ id: 'c', label: 'C' }
			],
			panel: createRawSnippet((id: () => string) => ({ render: () => `<p>painel ${id()}</p>` }))
		});
		const [a, b, c] = screen.getAllByRole('tab');
		expect(a).toHaveAttribute('aria-selected', 'true');
		expect(b).toHaveAttribute('tabindex', '-1');
		a.focus();
		await fireEvent.keyDown(a, { key: 'ArrowRight' });
		expect(b).toHaveAttribute('aria-selected', 'true');
		expect(document.activeElement).toBe(b);
		await fireEvent.keyDown(b, { key: 'End' });
		expect(document.activeElement).toBe(c);
		await fireEvent.keyDown(c, { key: 'ArrowRight' });
		expect(document.activeElement).toBe(a);
		await fireEvent.keyDown(a, { key: 'ArrowLeft' });
		expect(document.activeElement).toBe(c);
		expect(screen.getByRole('tabpanel')).toHaveTextContent('painel c');
	});
});

describe('Campos', () => {
	it('TextField associa label, dica e erro', () => {
		render(TextField, { label: 'Matrícula', hint: 'Só números', error: 'Obrigatório' });
		const input = screen.getByLabelText('Matrícula');
		expect(input).toHaveAttribute('aria-invalid', 'true');
		expect(input).toHaveAccessibleDescription('Só números ⚠ Obrigatório');
	});
	it('PasswordField mostra e oculta a senha', async () => {
		render(PasswordField, { label: 'Senha' });
		const input = screen.getByLabelText('Senha');
		expect(input).toHaveAttribute('type', 'password');
		const btn = screen.getByRole('button', { name: 'Mostrar senha' });
		await fireEvent.click(btn);
		expect(input).toHaveAttribute('type', 'text');
		expect(btn).toHaveAttribute('aria-pressed', 'true');
	});
	it('Select tem rótulo', () => {
		render(Select, { label: 'Tema', options: [{ value: 'a', label: 'A' }], value: 'a' });
		expect(screen.getByRole('combobox', { name: 'Tema' })).toBeInTheDocument();
	});
});

describe('DataTable', () => {
	const rows = [
		{ id: 1, t: 'B', n: 2 },
		{ id: 2, t: 'A', n: 10 }
	];
	it('ordena por coluna e expõe aria-sort', async () => {
		render(DataTable, {
			caption: 'Lista',
			rowKey: 'id',
			rows,
			columns: [
				{ key: 't', label: 'Título', sortable: true },
				{ key: 'n', label: 'Número', sortable: true, numeric: true }
			]
		});
		await fireEvent.click(screen.getByRole('button', { name: /Título/ }));
		let cells = screen.getAllByRole('cell');
		expect(cells[0]).toHaveTextContent('A');
		expect(screen.getAllByRole('columnheader')[0]).toHaveAttribute('aria-sort', 'ascending');
		await fireEvent.click(screen.getByRole('button', { name: /Título/ }));
		cells = screen.getAllByRole('cell');
		expect(cells[0]).toHaveTextContent('B');
	});
	it('mostra mensagem vazia', () => {
		render(DataTable, { caption: 'Lista', rowKey: 'id', rows: [], columns: [{ key: 'id', label: 'Id' }] });
		expect(screen.getByText('Nenhum registro.')).toBeInTheDocument();
	});
});

describe('Indicadores', () => {
	it('StatusPill não depende só de cor', () => {
		render(StatusPill, { status: 'danger', label: 'Atrasado' });
		expect(screen.getByText('Atrasado')).toBeInTheDocument();
		expect(screen.getByText('✕')).toHaveAttribute('aria-hidden', 'true');
	});
	it('ProgressSteps anuncia progresso em aria-live', () => {
		const { container } = render(ProgressSteps, {
			steps: [
				{ id: '1', label: 'Entrar', state: 'done' },
				{ id: '2', label: 'Renovar', state: 'active' }
			]
		});
		expect(container.querySelector('[aria-current="step"]')).toHaveTextContent('Renovar');
		expect(container.querySelector('[aria-live="polite"]')).toHaveTextContent('1 de 2 etapas concluídas. Renovar: em andamento');
	});
	it('Stamp formata a data', () => {
		render(Stamp, { date: '2026-11-06' });
		expect(screen.getByText('06/11/2026')).toHaveAttribute('datetime', '2026-11-06');
	});
	it('Spine com link tem nome acessível', () => {
		render(Spine, { title: 'Vidas Secas', author: 'Graciliano', href: '#x' });
		expect(screen.getByRole('link', { name: /Vidas Secas.*Graciliano/ })).toBeInTheDocument();
	});
	it('Skeleton é status ocupado', () => {
		render(Skeleton, {});
		expect(screen.getByRole('status')).toHaveAttribute('aria-busy', 'true');
	});
	it('EmptyState e Ficha renderizam títulos', () => {
		render(EmptyState, { title: 'Nada aqui' });
		render(Ficha, { title: 'Dom Casmurro', headingLevel: 3 });
		expect(screen.getByRole('heading', { level: 2, name: 'Nada aqui' })).toBeInTheDocument();
		expect(screen.getByRole('heading', { level: 3, name: 'Dom Casmurro' })).toBeInTheDocument();
	});
});

describe('Dialog', () => {
	it('abre com título e fecha pelo botão', async () => {
		const onclose = vi.fn();
		const { rerender } = render(Dialog, { open: false, title: 'Remover?', onclose });
		await rerender({ open: true, title: 'Remover?', onclose });
		const dlg = document.querySelector('dialog')!;
		expect(dlg.hasAttribute('open')).toBe(true);
		expect(dlg).toHaveAccessibleName('Remover?');
		await fireEvent.click(screen.getByRole('button', { name: 'Fechar', hidden: true }));
		expect(dlg.hasAttribute('open')).toBe(false);
	});
});

describe('Toast', () => {
	it('insere avisos numa região aria-live', () => {
		render(ToastRegion, {});
		const region = screen.getByRole('status');
		expect(region).toHaveAttribute('aria-live', 'polite');
		toast('Renovado.', 'ok', 0);
		flushSync();
		expect(region).toHaveTextContent('Sucesso: Renovado.');
		toasts.splice(0);
	});
});
