<script lang="ts">
	// 操作ログのウィンドウ（S03・S03a）。正本は本体のメモリ内のログで、追記・消去はイベントでその場に反映する
	import { onMount, tick } from 'svelte';
	import { m } from '#lib/i18n/index.js';
	import type { LogResult } from '#lib/api/types.js';
	import { retitleCurrentWindow } from '#lib/app/windows.js';
	import EmptyState from '#lib/components/EmptyState.svelte';
	import * as icons from '#lib/components/icons.js';
	import { formatTime } from '#lib/format/index.js';
	import { logStore } from '#lib/stores/index.js';

	const entries = $derived(logStore.entries);

	let scroller = $state<HTMLDivElement>();
	let copied = $state(false);
	let copiedTimer: ReturnType<typeof setTimeout> | undefined;

	function resultLabel(result: LogResult): string {
		switch (result) {
			case 'ok':
				return m['log.result.ok']();
			case 'fail':
				return m['log.result.fail']();
			case 'cancelled':
				return m['log.result.cancelled']();
		}
	}

	onMount(() => {
		void retitleCurrentWindow('log');
		void logStore.start();
		return () => clearTimeout(copiedTimer);
	});

	// 追記されたとき、いちばん下を見ていれば、新しい行が見えるように追いかける
	let followBottom = true;
	function handleScroll(): void {
		if (!scroller) return;
		followBottom = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 24;
	}
	$effect(() => {
		void entries.length;
		if (!followBottom) return;
		void tick().then(() => scroller?.scrollTo({ top: scroller.scrollHeight }));
	});

	async function copy(): Promise<void> {
		const lines = entries.map(
			(entry) => `${formatTime(new Date(entry.at))}\t${entry.command}\t${resultLabel(entry.result)}`
		);
		try {
			await navigator.clipboard.writeText(lines.join('\n'));
		} catch {
			return;
		}
		copied = true;
		clearTimeout(copiedTimer);
		copiedTimer = setTimeout(() => (copied = false), 2000);
	}
</script>

<main class="log">
	{#if entries.length === 0}
		<EmptyState icon={icons.log} title={m['log.empty.title']()} body={m['log.empty.body']()} />
	{:else}
		<div class="head">
			<span class="copied" role="status">{copied ? m['log.copied']() : ''}</span>
			<div class="actions">
				<cds-button kind="ghost" size="sm" onclick={() => void copy()}>{m['log.copy']()}</cds-button>
				<cds-button kind="ghost" size="sm" onclick={() => void logStore.clear()}>{m['log.clear']()}</cds-button>
			</div>
		</div>
		<div class="scroll" bind:this={scroller} onscroll={handleScroll}>
			<cds-table size="sm">
				<cds-table-head>
					<cds-table-header-row>
						<cds-table-header-cell>{m['log.col.time']()}</cds-table-header-cell>
						<cds-table-header-cell>{m['log.col.command']()}</cds-table-header-cell>
						<cds-table-header-cell>{m['log.col.result']()}</cds-table-header-cell>
					</cds-table-header-row>
				</cds-table-head>
				<cds-table-body>
					{#each entries as entry, index (index)}
						<cds-table-row>
							<cds-table-cell><span class="mono nowrap">{formatTime(new Date(entry.at))}</span></cds-table-cell>
							<cds-table-cell><span class="mono command">{entry.command}</span></cds-table-cell>
							<cds-table-cell><span class="nowrap {entry.result}">{resultLabel(entry.result)}</span></cds-table-cell>
						</cds-table-row>
					{/each}
				</cds-table-body>
			</cds-table>
		</div>
	{/if}
</main>

<style>
	.log {
		height: 100%;
		display: flex;
		flex-direction: column;
		background-color: var(--cds-layer-01);
	}

	.head {
		flex: none;
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--cds-spacing-03) var(--cds-spacing-05);
	}

	.copied {
		font-size: var(--cds-label-01-font-size);
		color: var(--cds-text-secondary);
	}

	.actions {
		display: flex;
		gap: var(--cds-spacing-03);
	}

	.scroll {
		flex: 1;
		min-height: 0;
		overflow: auto;
	}

	.mono {
		font-size: var(--cds-label-01-font-size);
	}

	.command {
		word-break: break-all;
	}

	.nowrap {
		white-space: nowrap;
	}

	/* 結果は色だけでなく文字でも示す */
	.ok {
		color: var(--esp-state-mounted);
	}

	.fail {
		color: var(--cds-support-error);
	}

	.cancelled {
		color: var(--cds-text-secondary);
	}
</style>
