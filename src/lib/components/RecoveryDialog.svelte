<script lang="ts">
	import { onMount } from "svelte";
	import { api, type RecoveryEntry } from "#lib/api";
	import { toastStore } from "#lib/stores/toast.svelte";
	import { downloadsStore } from "#lib/stores/downloads.svelte";
	import { t } from "#lib/i18n.svelte";

	let items = $state<RecoveryEntry[]>([]);
	let open = $state(false);
	let busy = $state(false);

	onMount(async () => {
		try {
			items = await api.getRecoveryItems();
			if (items.length > 0) open = true;
		} catch {
			items = [];
		}
	});

	async function restore() {
		busy = true;
		try {
			const restored = await api.restoreRecovery();
			toastStore.push(`${t("queue.statusQueued")}: ${restored}`);
			items = [];
			open = false;
			void downloadsStore.start();
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		} finally {
			busy = false;
		}
	}

	async function dismiss() {
		try {
			await api.dismissRecovery();
		} catch {
			/* silent */
		}
		open = false;
	}
</script>

{#if open}
	<div class="veil" role="dialog" aria-modal="true" aria-label={t("recovery.title")}>
		<div class="modal">
			<h2 class="ob-title">{t("recovery.title")}</h2>
			<p class="ob-body">{t("recovery.body")}</p>
			<div class="recovery-list">
				{#each items as item (item.id)}
					<p class="recovery-item mono" title={item.url}>{item.title}</p>
				{/each}
			</div>
			<div class="ob-foot">
				<button class="btn btn-ghost btn-sm" onclick={dismiss} disabled={busy}>{t("recovery.dismiss")}</button>
				<button class="btn btn-accent" onclick={restore} disabled={busy}>{t("recovery.restore")}</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.recovery-list {
		display: flex;
		flex-direction: column;
		gap: 6px;
		max-height: 160px;
		overflow-y: auto;
		border: 1px solid var(--border);
		border-radius: var(--r-sm);
		padding: 10px 12px;
	}

	.recovery-item {
		font-size: 11px;
		margin: 0;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		color: var(--muted);
	}
</style>
