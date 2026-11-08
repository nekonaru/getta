<script lang="ts">
	import { onMount, type Snippet } from "svelte";
	import "@fontsource-variable/fraunces";
	import "@fontsource-variable/work-sans";
	import "@fontsource/ibm-plex-mono/400.css";
	import "@fontsource/ibm-plex-mono/500.css";
	import "../app.css";

	import { page } from "$app/state";
	import Toaster from "#lib/components/Toaster.svelte";
	import CommandPalette from "#lib/components/CommandPalette.svelte";
	import Onboarding from "#lib/components/Onboarding.svelte";
	import LogoMark from "#lib/components/LogoMark.svelte";
	import { settingsStore } from "#lib/stores/settings.svelte";
	import { downloadsStore } from "#lib/stores/downloads.svelte";
	import { toastStore } from "#lib/stores/toast.svelte";
	import { enqueueFromClipboardText } from "#lib/enqueue";
	import { t } from "#lib/i18n.svelte";
	import { api } from "#lib/api";
	import { readText } from "@tauri-apps/plugin-clipboard-manager";
	import { onOpenUrl, getCurrent } from "@tauri-apps/plugin-deep-link";
	import { register, unregisterAll } from "@tauri-apps/plugin-global-shortcut";
	import pkg from "../../package.json";

	let { children }: { children: Snippet } = $props();

	let engineReady = $state<boolean | null>(null);

	const nav = [
		{ href: "/", key: "home", index: "01" },
		{ href: "/queue", key: "queue", index: "02" },
		{ href: "/library", key: "library", index: "03" },
		{ href: "/tools", key: "tools", index: "04" },
		{ href: "/settings", key: "settings", index: "05" }
	] as const;

	const activeCount = $derived(downloadsStore.activeCount + downloadsStore.queuedCount);

	onMount(() => {
		void settingsStore.load();
		void downloadsStore.start();
		void api.engineStatus().then((engine) => (engineReady = engine.ytdlp.found));

		// CmdOrCtrl+Shift+D: fetch from clipboard anywhere.
		void register("CmdOrCtrl+Shift+D", async () => {
			try {
				const text = await readText();
				await enqueueFromClipboardText(text ?? "");
			} catch {
				toastStore.push(t("home.invalidUrl"), "error");
			}
		}).catch((e) => console.debug("global shortcut unavailable", e));

		// getta://<url> links land here from the OS.
		void getCurrent().then((urls) => {
			for (const url of urls ?? []) void enqueueFromClipboardText(decodeURIComponent(url.replace(/^getta:\/\//, "")));
		});
		const unlisten = onOpenUrl(async (urls) => {
			for (const url of urls) {
				const decoded = (() => {
					try {
						return decodeURIComponent(url.replace(/^getta:\/\//, ""));
					} catch {
						return url.replace(/^getta:\/\//, "");
					}
				})();
				await enqueueFromClipboardText(decoded);
			}
		});

		return () => {
			void unregisterAll().catch(() => {});
			void unlisten.then((fn) => fn?.()).catch(() => {});
		};
	});
</script>

<div class="app-shell">
	<aside class="sidebar">
		<a class="wordmark" href="/">
			<LogoMark size={34} />
			<span>
				<span class="wordmark-name">getta</span>
				<span class="wordmark-meta">link in · file out</span>
			</span>
		</a>

		<nav aria-label="Main">
			{#each nav as item (item.href)}
				<a class="nav-item" href={item.href} aria-current={page.url.pathname === item.href ? "page" : undefined}>
					<span class="nav-index">{item.index}</span>
					<span class="nav-label">{t(`nav.${item.key}`)}</span>
					{#if item.key === "queue" && activeCount > 0}
						<span class="nav-count">{activeCount > 99 ? "99+" : activeCount}</span>
					{/if}
				</a>
			{/each}
		</nav>

		<div class="sidebar-foot">
			<a class="engine-chip" class:engine-ready={engineReady === true} href="/settings" style="text-decoration:none">
				<span class="engine-dot"></span>
				<span>{engineReady === false ? t("home.engineMissing") : "engine ready"}</span>
			</a>
			<div class="sidebar-links">
				<a href="https://github.com/nekonaru/getta" target="_blank" rel="noreferrer">GitHub</a>
				<a href="/about">v{pkg.version}</a>
			</div>
		</div>
	</aside>

	<main class="content">
		{@render children()}
	</main>

	<nav class="bottom-bar" aria-label="Main">
		{#each nav as item (item.href)}
			<a class="nav-item" href={item.href} aria-current={page.url.pathname === item.href ? "page" : undefined} aria-label={t(`nav.${item.key}`)}>
				<span class="nav-label">{t(`nav.${item.key}`)}</span>
				{#if item.key === "queue" && activeCount > 0}
					<span class="nav-count">{activeCount > 99 ? "99+" : activeCount}</span>
				{/if}
			</a>
		{/each}
	</nav>
</div>

<CommandPalette />
<Onboarding />
<Toaster />
