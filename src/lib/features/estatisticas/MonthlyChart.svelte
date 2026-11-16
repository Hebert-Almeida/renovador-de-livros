<script lang="ts">
	import type { MonthlyStat } from '#lib/ipc/bindings.ts';
	import t from '#lib/i18n/pt-BR/estatisticas.ts';
	import { formatInt, monthLabel, monthShort, niceScale } from './stats.ts';

	let { data }: { data: MonthlyStat[] } = $props();

	let width = $state(640);
	const height = 260;
	const m = { top: 24, right: 8, bottom: 32, left: 40 };
	const barMax = 24;
	const r = 4;

	const max = $derived(Math.max(0, ...data.map((d) => d.renewals)));
	const scale = $derived(niceScale(max));
	const innerW = $derived(Math.max(1, width - m.left - m.right));
	const innerH = height - m.top - m.bottom;
	const band = $derived(innerW / Math.max(1, data.length));
	const barW = $derived(Math.max(2, Math.min(barMax, band - 2)));
	const y = (v: number) => m.top + innerH - (v / scale.top) * innerH;
	const peakIndex = $derived(data.reduce((best, d, i) => (d.renewals > data[best].renewals ? i : best), 0));
	const total = $derived(data.reduce((s, d) => s + d.renewals, 0));
	const labelEvery = $derived(Math.max(1, Math.ceil(data.length / Math.max(1, Math.floor(innerW / 48)))));

	/** Coluna com topo arredondado (4px) e base reta no eixo. */
	function bar(x: number, v: number): string {
		const top = y(v);
		const base = y(0);
		const h = base - top;
		if (h <= 0) return '';
		const rr = Math.min(r, h, barW / 2);
		return `M${x},${base}V${top + rr}Q${x},${top} ${x + rr},${top}H${x + barW - rr}Q${x + barW},${top} ${x + barW},${top + rr}V${base}Z`;
	}

	let hover: number | null = $state(null);
	const tipX = $derived(hover == null ? 0 : m.left + band * hover + band / 2);
	/* Desloca o tooltip na proporção da posição (0 % na borda esquerda, -100 % na direita) para ele não vazar do gráfico. */
	const tipShift = $derived(width > 0 ? -Math.min(100, Math.max(0, (tipX / width) * 100)) : -50);
	const peak = $derived(data.length && data[peakIndex].renewals > 0 ? monthLabel(data[peakIndex].month) : null);
</script>

<div class="chart" bind:clientWidth={width}>
	<svg
		role="img"
		aria-label={data.length ? t.chartSummary(data.length, total, peak) : t.monthlyTitle}
		viewBox={`0 0 ${width} ${height}`}
		{width}
		{height}
	>
		<g aria-hidden="true">
			{#each scale.ticks as tick (tick)}
				<line class="grid" x1={m.left} x2={width - m.right} y1={y(tick)} y2={y(tick)} />
				<text class="axis" x={m.left - 8} y={y(tick)} dy="0.32em" text-anchor="end">{formatInt(tick)}</text>
			{/each}
			{#each data as d, i (d.month)}
				{@const x = m.left + band * i + (band - barW) / 2}
				<path class="bar" class:dim={hover != null && hover !== i} d={bar(x, d.renewals)} />
				{#if i === peakIndex && d.renewals > 0}
					<text class="value" x={x + barW / 2} y={y(d.renewals) - 6} text-anchor="middle">{formatInt(d.renewals)}</text>
				{/if}
				{#if i % labelEvery === 0}
					<text class="axis" x={m.left + band * i + band / 2} y={height - 10} text-anchor="middle">{monthShort(d.month)}</text>
				{/if}
				<!-- alvo de hover maior que a marca: a faixa inteira -->
				<rect
					class="hit"
					role="presentation"
					x={m.left + band * i}
					y={m.top}
					width={band}
					height={innerH}
					onpointerenter={() => (hover = i)}
					onpointerleave={() => (hover = null)}
				/>
			{/each}
			<line class="baseline" x1={m.left} x2={width - m.right} y1={y(0)} y2={y(0)} />
		</g>
	</svg>
	{#if hover != null && data[hover]}
		{@const d = data[hover]}
		<div class="tip" aria-hidden="true" style:left={`${tipX}px`} style:top={`${y(d.renewals)}px`} style:--tip-shift={`${tipShift}%`}>
			<strong>{monthLabel(d.month)}</strong>
			<span>{t.tooltip(d.renewals, d.successes, d.failures)}</span>
		</div>
	{/if}
</div>

<style>
	.chart {
		position: relative;
		width: 100%;
		--chart-bar: var(--color-accent);
	}
	/* Escuro: o acento puro fica claro demais para marca de dados (validado com a skill dataviz). */
	@media (prefers-color-scheme: dark) {
		:global(:root:not([data-theme='light']):not([data-theme='contrast'])) .chart {
			--chart-bar: color-mix(in oklab, var(--color-accent) 80%, black);
		}
	}
	:global(:root[data-theme='dark']) .chart {
		--chart-bar: color-mix(in oklab, var(--color-accent) 80%, black);
	}
	svg {
		display: block;
		max-width: 100%;
		overflow: visible;
	}
	.bar {
		fill: var(--chart-bar);
		transition: opacity var(--duration-fast) var(--ease-out);
	}
	.bar.dim {
		opacity: 0.45;
	}
	.grid {
		stroke: var(--color-ink-muted);
		stroke-opacity: 0.2;
		stroke-width: 1;
	}
	.baseline {
		stroke: var(--color-rule);
		stroke-width: 2;
	}
	.axis {
		fill: var(--color-ink-muted);
		font-family: var(--font-mono);
		font-size: 12px;
	}
	.value {
		fill: var(--color-ink);
		font-family: var(--font-mono);
		font-size: 12px;
		font-weight: var(--weight-bold);
	}
	.hit {
		fill: transparent;
	}
	.tip {
		position: absolute;
		transform: translate(var(--tip-shift, -50%), calc(-100% - 0.75rem));
		display: grid;
		gap: var(--space-1);
		padding: var(--space-2) var(--space-3);
		background: var(--color-paper-raised);
		color: var(--color-ink);
		border: var(--border-width-thin) solid var(--color-rule);
		box-shadow: var(--shadow-hard);
		font-size: var(--text-xs);
		white-space: nowrap;
		pointer-events: none;
	}
	@media (prefers-reduced-motion: reduce) {
		.bar {
			transition: none;
		}
	}
	@media (forced-colors: active) {
		.bar {
			fill: CanvasText;
		}
	}
</style>
