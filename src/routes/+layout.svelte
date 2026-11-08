<script lang="ts">
	import { onMount, type Snippet } from "svelte";
	import "@fontsource-variable/fraunces";
	import "@fontsource-variable/work-sans";
	import "@fontsource/ibm-plex-mono/400.css";
	import "@fontsource/ibm-plex-mono/500.css";
	import "../app.css";

	import { page } from "$app/state";
	import Toaster from "$lib/components/Toaster.svelte";
	import LogoMark from "$lib/components/LogoMark.svelte";
	import { settingsStore } from "$lib/stores/settings.svelte";
	import { downloadsStore } from "$lib/stores/downloads.svelte";
	import { t } from "$lib/i18n";

	let { children }: { children: Snippet } = $props();

	onMount(() => {
		void settingsStore.load();
		void downloadsStore.start();
	});

	const nav = [
		{ href: "/", key: "home" as const },
		{ href: "/downloads", key: "downloads" as const },
		{ href: "/settings", key: "settings" as const },
		{ href: "/about", key: "about" as const }
	];

	const activeCount = $derived(downloadsStore.activeCount);
</script>

<div class="app-shell">
	<nav class="rail" aria-label="Main navigation">
		<a class="rail-logo" href="/" aria-label="Getta">
			<LogoMark size={26} />
		</a>
		<div class="rail-nav">
			{#each nav as item (item.href)}
				<a
					class="rail-item"
					href={item.href}
					aria-current={page.url.pathname === item.href ? "page" : undefined}
				>
					{#if item.key === "home"}
						<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 11.5 12 4l8 7.5" /><path d="M6.5 10v9h11v-9" /></svg>
					{:else if item.key === "downloads"}
						<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4v10" /><path d="m7.5 10.5 4.5 4.5 4.5-4.5" /><path d="M4.5 19h15" /></svg>
					{:else if item.key === "settings"}
						<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="4.2" /><path d="M12 2.5v3.5M12 18v3.5M2.5 12H6M18 12h3.5M5 5l2.5 2.5M16.5 16.5 19 19M19 5l-2.5 2.5M7.5 16.5 5 19" /></svg>
					{:else}
						<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9" /><path d="M12 11v5.5" /><circle cx="12" cy="7.6" r="0.4" fill="currentColor" stroke="none" /></svg>
					{/if}
					<span>{t(`nav.${item.key}`)}</span>
					{#if item.key === "downloads" && activeCount > 0}
						<span class="rail-badge">{activeCount > 99 ? "99+" : activeCount}</span>
					{/if}
				</a>
			{/each}
		</div>
	</nav>

	<main class="content">
		{@render children()}
	</main>

	<nav class="bottom-bar" aria-label="Main navigation">
		{#each nav as item (item.href)}
			<a
				class="rail-item"
				href={item.href}
				aria-current={page.url.pathname === item.href ? "page" : undefined}
			>
				{#if item.key === "home"}
					<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 11.5 12 4l8 7.5" /><path d="M6.5 10v9h11v-9" /></svg>
				{:else if item.key === "downloads"}
					<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4v10" /><path d="m7.5 10.5 4.5 4.5 4.5-4.5" /><path d="M4.5 19h15" /></svg>
				{:else if item.key === "settings"}
					<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="4.2" /><path d="M12 2.5v3.5M12 18v3.5M2.5 12H6M18 12h3.5M5 5l2.5 2.5M16.5 16.5 19 19M19 5l-2.5 2.5M7.5 16.5 5 19" /></svg>
				{:else}
					<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9" /><path d="M12 11v5.5" /><circle cx="12" cy="7.6" r="0.4" fill="currentColor" stroke="none" /></svg>
				{/if}
				<span>{t(`nav.${item.key}`)}</span>
				{#if item.key === "downloads" && activeCount > 0}
					<span class="rail-badge">{activeCount > 99 ? "99+" : activeCount}</span>
				{/if}
			</a>
		{/each}
	</nav>
</div>

<Toaster />
