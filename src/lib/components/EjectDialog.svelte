<script lang="ts">
	// デバイスの取り出しの確認（S01i）：Carbonのモーダル。ESP単体の取り外しは確認しない。
	// 閉じる操作（キャンセル・Esc・外側のクリック）はonclose、「取り出す」はonconfirmで知らせる
	import { m } from '#lib/i18n/index.js';
	import type { Device } from '#lib/api/types.js';
	import { displayNames, ejectVolumeCount } from '#lib/devices/model.js';

	interface Props {
		device: Device;
		open: boolean;
		onconfirm: () => void;
		onclose: () => void;
	}

	let { device, open, onconfirm, onclose }: Props = $props();

	const name = $derived(displayNames(device).main);
	const count = $derived(ejectVolumeCount(device));
</script>

<cds-modal {open} size="xs" aria-label={m['modal.eject.label']()} oncds-modal-closed={onclose}>
	<cds-modal-header>
		<cds-modal-label>{m['modal.eject.label']()}</cds-modal-label>
		<cds-modal-heading>{m['modal.eject.title']({ device: name })}</cds-modal-heading>
	</cds-modal-header>
	<cds-modal-body>
		<p>{m['modal.eject.body']({ count })}</p>
	</cds-modal-body>
	<cds-modal-footer>
		<cds-modal-footer-button kind="secondary" data-modal-close>{m['modal.cancel']()}</cds-modal-footer-button>
		<cds-modal-footer-button kind="primary" onclick={onconfirm}>{m['modal.eject.confirm']()}</cds-modal-footer-button>
	</cds-modal-footer>
</cds-modal>
