<script lang="ts">
	import { toastStore } from "#lib/stores/toast.svelte";
</script>

{#if toastStore.items.length > 0}
	<div class="toast-region" aria-live="polite">
		{#each toastStore.items as toast (toast.id)}
			<div class="toast {toast.kind === 'error' ? 'error' : ''}" role="status">
				<span style="flex: 1">{toast.message}</span>
				<button class="toast-close" aria-label="Close" onclick={() => toastStore.dismiss(toast.id)}>×</button>
			</div>
		{/each}
	</div>
{/if}

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
