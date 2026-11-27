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
	import { onOpenUrl, getCurrent } from "@tauri-apps/plugin-deep-link";
	import pkg from "../../package.json";

	let { children }: { children: Snippet } = $props();

	let engineReady = $state<boolean | null>(null);
	let dragDepth = $state(0);
	const compact = $derived(settingsStore.value?.compact_mode === true);

	function onDragEnter(event: DragEvent) {
		event.preventDefault();
		dragDepth += 1;
	}

	function onDragOver(event: DragEvent) {
		event.preventDefault();
		if (event.dataTransfer) event.dataTransfer.dropEffect = "copy";
	}

	function onDragLeave(event: DragEvent) {
		event.preventDefault();
		dragDepth = Math.max(0, dragDepth - 1);
	}

	async function onDrop(event: DragEvent) {
		event.preventDefault();
		dragDepth = 0;
			const text =
			event.dataTransfer?.getData("text/uri-list") ||
			event.dataTransfer?.getData("text/plain") ||
			"";
		// A dropped URI list can carry several links. Extract each one so none
		// are silently dropped.
		const tokens = text
			.split(/[\r\n]+/)
			.flatMap((line) => line.split(/\s+/))
			.map((token) => token.trim())
			.filter((token) => /^(https?:\/\/|magnet:|p2p:)/i.test(token));
		for (const token of tokens) await enqueueFromClipboardText(token);
	}

	const nav = [
		{ href: "/", key: "home", index: "01" },
		{ href: "/queue", key: "queue", index: "02" },
		{ href: "/follows", key: "follows", index: "03" },
		{ href: "/library", key: "library", index: "04" },
		{ href: "/tools", key: "tools", index: "05" },
		{ href: "/settings", key: "settings", index: "06" }
	] as const;

	const activeCount = $derived(downloadsStore.activeCount + downloadsStore.queuedCount);
	const hasFailed = $derived(downloadsStore.failedCount > 0);

	onMount(() => {
		void settingsStore.load();
		void downloadsStore.start();
		void api.engineStatus().then((engine) => (engineReady = engine.ytdlp.found));
		void api.takePendingLinks().then((urls) => {
			for (const url of urls) void handleDeepUrl(url);
		}).catch(() => {});
		void api.takePendingVerbs().then((verbs) => {
			for (const verb of verbs) {
				if (verb === "pause") void api.pauseAll().then(() => downloadsStore.refresh());
				else if (verb === "resume") void api.resumeAll().then(() => downloadsStore.refresh());
			}
		}).catch(() => {});

		const handledDeep = new Map<string, number>();
		const handleEnqueue = async (raw: string, key = raw) => {
			const now = Date.now();
			for (const [k, at] of handledDeep) {
				if (now - at > 2500) handledDeep.delete(k);
			}
			if (handledDeep.has(key)) return;
			handledDeep.set(key, now);
			await enqueueFromClipboardText(raw);
		};
		const handleDeepUrl = (raw: string) => {
			const decoded = (() => {
				try {
					return decodeURIComponent(raw.replace(/^getta:\/\//, ""));
				} catch {
					return raw.replace(/^getta:\/\//, "");
				}
			})();
			return handleEnqueue(decoded);
		};

		// CmdOrCtrl+Shift+D is registered on the Rust side and arrives as an event.
		const unlistenGlobalShortcut = listen("global-download-requested", async () => {
			try {
				const text = await api.getClipboardText();
				await handleEnqueue(text);
			} catch (e) {
				const detail = e instanceof Error ? e.message : String(e);
				toastStore.push(detail || t("enqueue.emptyClipboard"), "error");
			}
		});

		// Second-instance deep links forward here as an event.
		const unlistenDeepLinkRequest = listen<string>("deep-link-request", (event) => {
			void handleDeepUrl(event.payload);
		});
		const unlistenEngineBroken = listen("engine-broken", () => {
			toastStore.push(t("layout.engineBroken"), "error");
		});
		const unlistenCliPause = listen("cli-pause", () => {
			void api.pauseAll().then(() => downloadsStore.start());
		});
		const unlistenCliResume = listen("cli-resume", () => {
			void api.resumeAll().then(() => downloadsStore.start());
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
			void unlistenEngineBroken.then((fn) => fn?.()).catch(() => {});
			void unlistenCliPause.then((fn) => fn?.()).catch(() => {});
			void unlistenCliResume.then((fn) => fn?.()).catch(() => {});
			void unlisten.then((fn) => fn?.()).catch(() => {});
		};
	});
</script>

<div class="app-shell" class:drop-active={dragDepth > 0} class:compact-shell={compact} role="group" aria-label={t("layout.dropHint")} ondragenter={onDragEnter} ondragover={onDragOver} ondragleave={onDragLeave} ondrop={onDrop}>
	<aside class="sidebar">
		<a class="wordmark" href="/">
			<LogoMark size={34} />
			<span>
				<span class="wordmark-name">getta</span>
				<span class="wordmark-meta">{t("layout.tagline")}</span>
			</span>
		</a>

		<nav aria-label="Main">
			{#each nav as item (item.href)}
				<a class="nav-item" href={item.href} aria-current={page.url.pathname === item.href ? "page" : undefined}>
					<span class="nav-index">{item.index}</span>
					<span class="nav-label">{t(`nav.${item.key}`)}</span>
					{#if item.key === "queue" && activeCount > 0}
						<span class="nav-count" class:failed={hasFailed}>{activeCount > 99 ? "99+" : activeCount}</span>
					{/if}
				</a>
			{/each}
		</nav>

		<div class="sidebar-foot">
			<a class="engine-chip" class:engine-ready={engineReady === true} class:engine-checking={engineReady === null} href="/settings" style="text-decoration:none">
				<span class="engine-dot"></span>
				<span>{engineReady === null ? t("home.engineChecking") : engineReady ? t("layout.engineReady") : t("home.engineMissing")}</span>
			</a>
			<div class="sidebar-links">
				<button class="sidebar-link" onclick={() => settingsStore.update({ compact_mode: !compact })}>{compact ? t("layout.normalMode") : t("layout.focusMode")}</button>
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
					<span class="nav-count" class:failed={hasFailed}>{activeCount > 99 ? "99+" : activeCount}</span>
				{/if}
			</a>
		{/each}
	</nav>

	{#if dragDepth > 0}
		<div class="drop-overlay" aria-hidden="true">
			<div class="drop-card">
				<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 16V4m0 0L7 9m5-5 5 5M5 16v3a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-3" /></svg>
				<span>{t("layout.dropHint")}</span>
			</div>
		</div>
	{/if}
</div>

<CommandPalette />
<Onboarding />
<RecoveryDialog />
<Toaster />
