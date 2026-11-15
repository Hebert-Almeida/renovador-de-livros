<script lang="ts" module>
	export interface Column<T> {
		key: keyof T & string;
		label: string;
		sortable?: boolean;
		numeric?: boolean;
		mono?: boolean;
	}
</script>

<script lang="ts" generics="T extends Record<string, unknown>">
	let {
		caption,
		columns,
		rows,
		rowKey,
		empty = 'Nenhum registro.'
	}: { caption: string; columns: Column<T>[]; rows: T[]; rowKey: keyof T & string; empty?: string } = $props();

	let sortKey: string | null = $state(null);
	let sortDir: 'ascending' | 'descending' = $state('ascending');

	const sorted = $derived.by(() => {
		if (!sortKey) return rows;
		const k = sortKey;
		const dir = sortDir === 'ascending' ? 1 : -1;
		return [...rows].sort((a, b) => {
			const x = a[k];
			const y = b[k];
			if (x == null || y == null) return x == null ? (y == null ? 0 : 1) : -1;
			if (typeof x === 'number' && typeof y === 'number') return (x - y) * dir;
			return String(x).localeCompare(String(y), 'pt-BR') * dir;
		});
	});

	function sortBy(key: string) {
		if (sortKey === key) sortDir = sortDir === 'ascending' ? 'descending' : 'ascending';
		else {
			sortKey = key;
			sortDir = 'ascending';
		}
	}
</script>

<!-- Região rolável precisa ser focável para rolar via teclado (axe: scrollable-region-focusable). -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div class="table-wrap" role="region" aria-label={caption} tabindex="0">
	<table class="table">
		<caption class="table__caption">{caption}</caption>
		<thead>
			<tr>
				{#each columns as c (c.key)}
					<th scope="col" class:num={c.numeric} aria-sort={sortKey === c.key ? sortDir : undefined}>
						{#if c.sortable}
							<button type="button" class="table__sort" onclick={() => sortBy(c.key)}>
								{c.label}
								<span aria-hidden="true">{sortKey === c.key ? (sortDir === 'ascending' ? '▲' : '▼') : '↕'}</span>
							</button>
						{:else}
							{c.label}
						{/if}
					</th>
				{/each}
			</tr>
		</thead>
		<tbody>
			{#each sorted as r (r[rowKey])}
				<tr>
					{#each columns as c (c.key)}
						<td class:num={c.numeric} class:mono={c.mono}>{r[c.key] ?? '—'}</td>
					{/each}
				</tr>
			{:else}
				<tr><td colspan={columns.length} class="table__empty">{empty}</td></tr>
			{/each}
		</tbody>
	</table>
</div>

<style>
	.table-wrap {
		overflow-x: auto;
		border: var(--border);
		box-shadow: var(--shadow-hard);
		background: var(--color-paper-raised);
	}
	.table {
		width: 100%;
		border-collapse: collapse;
	}
	.table__caption {
		text-align: left;
		padding: var(--space-2) var(--space-3);
		font-family: var(--font-display);
		font-weight: var(--weight-bold);
		border-bottom: var(--border);
	}
	th,
	td {
		padding: var(--space-2) var(--space-3);
		text-align: left;
		border-bottom: var(--border-width-thin) solid var(--color-rule);
	}
	th {
		font-family: var(--font-display);
		font-size: var(--text-sm);
		text-transform: uppercase;
		letter-spacing: var(--tracking-caps);
		background: var(--color-paper-sunken);
	}
	.num {
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
	.mono {
		font-family: var(--font-mono);
	}
	.table__sort {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		min-height: var(--target-min);
		padding: 0;
		border: 0;
		background: none;
		color: inherit;
		font: inherit;
		text-transform: inherit;
		letter-spacing: inherit;
		cursor: pointer;
	}
	.table__empty {
		text-align: center;
		color: var(--color-ink-muted);
	}
</style>
