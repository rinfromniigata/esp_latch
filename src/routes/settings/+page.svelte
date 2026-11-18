<script lang="ts">
	// 設定のウィンドウ（S02）。保存ボタンはなく、1項目ずつ即時に保存する。
	// 変更は本体が全ウィンドウにsettings-changedで配り、メインウィンドウも同時に切り替わる
	import { onMount } from 'svelte';
	import { m } from '#lib/i18n/index.js';
	import { appInfo, openRepository, type AppInfo } from '#lib/api/commands.js';
	import type { DefaultFilter, Settings } from '#lib/api/types.js';
	import { retitleCurrentWindow } from '#lib/app/windows.js';
	import { filterLabel } from '#lib/devices/labels.js';
	import { settingsStore } from '#lib/stores/index.js';

	type BooleanKey =
		| 'open_finder_after_mount'
		| 'auto_refresh'
		| 'confirm_eject'
		| 'auto_empty_trash_on_unmount'
		| 'show_command';

	const settings = $derived(settingsStore.settings);

	const languages = $derived([
		{ value: 'system', label: m['settings.language.system']() },
		{ value: 'ja', label: m['settings.language.ja']() },
		{ value: 'en', label: m['settings.language.en']() },
		{ value: 'zh-Hans', label: m['settings.language.zh_hans']() }
	]);
	const themes = $derived([
		{ value: 'system', label: m['settings.theme.system']() },
		{ value: 'light', label: m['settings.theme.light']() },
		{ value: 'dark', label: m['settings.theme.dark']() }
	]);
	const filters = $derived(
		(['all', 'esp', 'physical'] as const satisfies readonly DefaultFilter[]).map((value) => ({
			value,
			label: filterLabel(value)
		}))
	);
	const toggles = $derived<{ key: BooleanKey; label: string }[]>([
		{ key: 'open_finder_after_mount', label: m['settings.open_finder']() },
		{ key: 'auto_refresh', label: m['settings.auto_refresh']() },
		{ key: 'confirm_eject', label: m['settings.confirm_eject']() },
		{ key: 'auto_empty_trash_on_unmount', label: m['settings.auto_empty_trash']() },
		{ key: 'show_command', label: m['settings.show_command']() }
	]);

	let info = $state<AppInfo | null>(null);

	onMount(() => {
		void retitleCurrentWindow('settings');
		void appInfo().then((value) => (info = value)).catch(() => undefined);
	});

	function save(patch: Partial<Settings>): void {
		// 保存に失敗しても、次のsettings-changedか読み込みで本体の値に戻る
		void settingsStore.update(patch).catch(() => undefined);
	}

	function choose<K extends 'language' | 'theme' | 'default_filter'>(key: K, options: { value: string }[]) {
		return (event: Event): void => {
			const value = (event as CustomEvent<{ value: string }>).detail.value;
			if (options.some((option) => option.value === value) && value !== settings[key]) {
				save({ [key]: value } as Partial<Settings>);
			}
		};
	}

	function toggle(key: BooleanKey) {
		return (event: Event): void => {
			const checked = (event as CustomEvent<{ checked: boolean }>).detail.checked;
			if (checked !== settings[key]) save({ [key]: checked });
		};
	}

	function displayUrl(url: string): string {
		return url.replace(/^https?:\/\//, '');
	}

	function openRepo(event: Event): void {
		event.preventDefault();
		void openRepository().catch(() => undefined);
	}
</script>

{#snippet select(
	label: string,
	key: 'language' | 'theme' | 'default_filter',
	options: { value: string; label: string }[]
)}
	<div class="item">
		<span class="label" aria-hidden="true">{label}</span>
		<cds-select
			class="control"
			size="md"
			hide-label
			label-text={label}
			value={settings[key]}
			oncds-select-selected={choose(key, options)}
		>
			{#each options as option (option.value)}
				<cds-select-item value={option.value} selected={option.value === settings[key]}>
					{option.label}
				</cds-select-item>
			{/each}
		</cds-select>
	</div>
{/snippet}

<main class="settings">
	<section class="group" aria-labelledby="group-general">
		<h2 id="group-general">{m['settings.group.general']()}</h2>
		{@render select(m['settings.language.label'](), 'language', languages)}
		{@render select(m['settings.theme.label'](), 'theme', themes)}
		{@render select(m['settings.default_filter'](), 'default_filter', filters)}
	</section>

	<section class="group" aria-labelledby="group-behavior">
		<h2 id="group-behavior">{m['settings.group.behavior']()}</h2>
		{#each toggles as item (item.key)}
			<div class="item">
				<cds-toggle
					size="sm"
					hideLabel
					label-text={item.label}
					toggled={settings[item.key]}
					oncds-toggle-changed={toggle(item.key)}
				></cds-toggle>
			</div>
		{/each}
	</section>

	<section class="group" aria-labelledby="group-about">
		<h2 id="group-about">{m['settings.group.about']()}</h2>
		<div class="item">
			<span>{m['app.name']()}</span>
			<span class="mono muted">{info ? m['settings.version']({ version: info.version }) : ''}</span>
		</div>
		<div class="item">
			<span>{m['settings.repository']()}</span>
			{#if info}
				<cds-link class="mono" href={info.repository} onclick={openRepo}>{displayUrl(info.repository)}</cds-link>
			{/if}
		</div>
	</section>
</main>

<style>
	.settings {
		min-height: 100%;
		box-sizing: border-box;
		padding: var(--cds-spacing-06);
		display: flex;
		flex-direction: column;
		gap: var(--cds-spacing-06);
		background-color: var(--cds-layer-01);
	}

	h2 {
		margin: 0 0 var(--cds-spacing-04);
		font-size: var(--cds-label-01-font-size);
		font-weight: 600;
		color: var(--cds-text-secondary);
	}

	.item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: var(--cds-spacing-05);
		min-height: 3rem;
		padding: var(--cds-spacing-03) 0;
		border-bottom: 1px solid var(--cds-border-subtle-01);
		font-size: var(--cds-body-compact-01-font-size);
	}

	.group .item:last-child {
		border-bottom: 0;
	}

	/* スイッチのオンの色は、マウント中と同じブランドの色（モックと同じ） */
	cds-toggle {
		--cds-support-success: var(--esp-state-mounted);
	}

	.control {
		flex: none;
		width: 12.5rem;
	}

	.muted {
		color: var(--cds-text-secondary);
	}

	.mono {
		font-size: var(--cds-label-01-font-size);
	}
</style>
