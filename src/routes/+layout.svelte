<script lang="ts">
	import { onMount, type Snippet } from "svelte";
	import "@fontsource-variable/fraunces";
	import "@fontsource-variable/work-sans";
	import "@fontsource/ibm-plex-mono/400.css";
	import "@fontsource/ibm-plex-mono/500.css";
	import "../app.css";

	import { listen } from "@tauri-apps/api/event";

	import { page } from "$app/state";
	import { openExternal } from "#lib/external";
	import Toaster from "#lib/components/Toaster.svelte";
	import CommandPalette from "#lib/components/CommandPalette.svelte";
	import Onboarding from "#lib/components/Onboarding.svelte";
	import RecoveryDialog from "#lib/components/RecoveryDialog.svelte";
	import LogoMark from "#lib/components/LogoMark.svelte";
	import { settingsStore } from "#lib/stores/settings.svelte";
	import { downloadsStore } from "#lib/stores/downloads.svelte";
	import { toastStore } from "#lib/stores/toast.svelte";
	import { enqueueFromClipboardText } from "#lib/enqueue";
	import { t } from "#lib/i18n.svelte";
	import { api } from "#lib/api";
	import { readText } from "@tauri-apps/plugin-clipboard-manager";
	import { onOpenUrl, getCurrent } from "@tauri-apps/plugin-deep-link";
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

		const handledDeep = new Map<string, number>();
		const handleDeepUrl = async (raw: string) => {
			const decoded = (() => {
				try {
					return decodeURIComponent(raw.replace(/^getta:\/\//, ""));
				} catch {
					return raw.replace(/^getta:\/\//, "");
				}
			})();
			const now = Date.now();
			for (const [key, at] of handledDeep) {
				if (now - at > 2500) handledDeep.delete(key);
			}
			if (handledDeep.has(decoded)) return;
			handledDeep.set(decoded, now);
			await enqueueFromClipboardText(decoded);
		};

		// CmdOrCtrl+Shift+D is registered on the Rust side and arrives as an event.
		const unlistenGlobalShortcut = listen("global-download-requested", async () => {
			try {
				const text = await readText();
				await enqueueFromClipboardText(text ?? "");
			} catch {
				toastStore.push(t("home.invalidUrl"), "error");
			}
		});

		// Second-instance deep links forward here as an event.
		const unlistenDeepLinkRequest = listen<string>("deep-link-request", (event) => {
			void handleDeepUrl(event.payload);
		});

		// getta://<url> links land here from the OS.
		void getCurrent().then((urls) => {
			for (const url of urls ?? []) void handleDeepUrl(url);
		});
		const unlisten = onOpenUrl(async (urls) => {
			for (const url of urls) await handleDeepUrl(url);
		});

		return () => {
			void unlistenGlobalShortcut.then((fn) => fn?.()).catch(() => {});
			void unlistenDeepLinkRequest.then((fn) => fn?.()).catch(() => {});
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
			<a class="engine-chip" class:engine-ready={engineReady === true} class:engine-checking={engineReady === null} href="/settings" style="text-decoration:none">
				<span class="engine-dot"></span>
				<span>{engineReady === null ? t("home.engineChecking") : engineReady ? "engine ready" : t("home.engineMissing")}</span>
			</a>
			<div class="sidebar-links">
				<button class="sidebar-link" onclick={() => openExternal("https://github.com/nekonaru/getta")}>GitHub</button>
				<a class="sidebar-link" href="/support">{t("nav.support")}</a>
				<a class="sidebar-link" href="/about">v{pkg.version}</a>
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
<RecoveryDialog />
<Toaster />
