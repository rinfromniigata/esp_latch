<script lang="ts">
	// 通知（右下）：Carbonのトースト通知。成功は数秒で消え、エラーは閉じるまで残る。
	// コマンドの失敗は、標準エラー出力と実行したコマンドを原文のまま出す（S01h）。ボタンの動作は通知を出した側が決める
	import { m } from '#lib/i18n/index.js';
	import { toastStore, type Toast, type ToastActionId } from '#lib/stores/index.js';

	function actionLabel(action: ToastActionId): string {
		return action === 'retry' ? m['toast.retry']() : m['toast.show_log']();
	}

	function body(toast: Toast): string {
		return toast.body ?? toast.stderr ?? '';
	}
</script>

<div class="layer">
	{#each toastStore.items as toast (toast.id)}
		<cds-toast-notification
			kind={toast.kind === 'ok' ? 'success' : 'error'}
			role={toast.kind === 'ok' ? 'status' : 'alert'}
			title={toast.title}
			subtitle={body(toast)}
			aria-label={m['toast.close']()}
			oncds-notification-closed={() => toastStore.dismiss(toast.id)}
		>
			{#if toast.command}<div class="command">{toast.command}</div>{/if}
			{#if toast.actions.length > 0}
				<div class="actions">
					{#each toast.actions as action, index (action)}
						<cds-button
							kind={index === 0 ? 'tertiary' : 'ghost'}
							size="sm"
							onclick={() => toast.onAction?.(action)}
						>
							{actionLabel(action)}
						</cds-button>
					{/each}
				</div>
			{/if}
		</cds-toast-notification>
	{/each}
</div>

<style>
	.layer {
		position: fixed;
		right: var(--cds-spacing-05);
		bottom: calc(1.75rem + var(--cds-spacing-05));
		z-index: 10;
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: var(--cds-spacing-03);
		max-width: calc(100% - 2 * var(--cds-spacing-05));
	}

	cds-toast-notification {
		max-width: 27.5rem;
	}

	/* 実行したコマンド（原文）。先頭の「$」はシェルのプロンプトの記号で、文言ではない */
	.command {
		margin-bottom: var(--cds-spacing-03);
		font-family: var(--esp-font-mono);
		font-size: var(--cds-code-01-font-size);
		color: var(--cds-text-secondary);
		overflow-wrap: anywhere;
	}

	.command::before {
		content: '$ ';
	}

	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: var(--cds-spacing-03);
		margin-bottom: var(--cds-spacing-04);
	}
</style>
