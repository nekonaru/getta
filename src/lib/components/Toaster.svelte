<script lang="ts">
	import { toastStore } from "#lib/stores/toast.svelte";
	import { t } from "#lib/i18n.svelte";
</script>

<div class="toast-region" aria-live="polite">
	{#each toastStore.items as toast (toast.id)}
		<div
			class="toast {toast.kind === 'error' ? 'error' : ''}"
			role={toast.kind === 'error' ? "alert" : "status"}
		>
			<span style="flex: 1">{toast.message}</span>
			<button class="toast-close" aria-label={t("common.close")} onclick={() => toastStore.dismiss(toast.id)}>×</button>
		</div>
	{/each}
</div>

<style>
	.toast-close {
		background: transparent;
		border: none;
		color: inherit;
		font-size: 18px;
		line-height: 1;
		cursor: pointer;
		padding: 0 4px;
		opacity: 0.8;
	}
	.toast-close:hover {
		opacity: 1;
	}
</style>
