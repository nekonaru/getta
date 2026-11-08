<script lang="ts">
	import { onMount } from "svelte";
	import { goto } from "$app/navigation";
	import { open } from "@tauri-apps/plugin-dialog";
	import { api, type Analysis, type EngineStatus } from "$lib/api";
	import { downloadsStore } from "$lib/stores/downloads.svelte";
	import { toastStore } from "$lib/stores/toast.svelte";
	import { settingsStore } from "$lib/stores/settings.svelte";
	import { t } from "$lib/i18n";
	import { formatBytes, formatDuration } from "$lib/format";
	import LogoMark from "$lib/components/LogoMark.svelte";

	let url = $state("");
	let busy = $state<"idle" | "analyzing" | "ready" | "starting" | "error">("idle");
	let analysis = $state<Analysis | null>(null);
	let errorMessage = $state("");
	let engine = $state<EngineStatus | null>(null);
	let engineInstalling = $state(false);
	let quality = $state("best");
	let mode = $state<"auto" | "audio" | "mute">("auto");
	let analyzeTimer: ReturnType<typeof setTimeout> | undefined;

	const links = $derived(parseLinks(url));
	const singleLink = $derived(links.length === 1 ? links[0] : null);

	const recentItems = $derived(
		[...downloadsStore.items]
			.sort((a, b) => b.id - a.id)
			.slice(0, 3)
	);

	onMount(async () => {
		try {
			engine = await api.engineStatus();
		} catch {
			engine = null;
		}
	});

	function parseLinks(text: string): string[] {
		const tokens = text
			.split(/\s+/)
			.map((t) => t.trim())
			.filter((t) => /^(https?:\/\/|magnet:|p2p:)/.test(t));
		const unique = [...new Set(tokens)];
		return unique.slice(0, 200);
	}

	function looksLikeUrl(text: string): boolean {
		return /^(https?:\/\/|magnet:|p2p:).+/.test(text.trim());
	}

	function onInput() {
		clearTimeout(analyzeTimer);
		analysis = null;
		if (!looksLikeUrl(url)) {
			busy = "idle";
			errorMessage = "";
			return;
		}
		busy = "analyzing";
		errorMessage = "";
		analyzeTimer = setTimeout(() => void analyze(), 550);
	}

	async function analyze() {
		const target = singleLink;
		if (!target) return;
		try {
			const result = await api.analyzeUrl(target);
			analysis = result;
			busy = "ready";
			quality = "best";
		} catch (e) {
			busy = "error";
			errorMessage = e instanceof Error ? e.message : String(e);
		}
	}

	async function startDownload() {
		if (!singleLink) return;
		busy = "starting";
		try {
			const alwaysAsk = settingsStore.value?.always_ask_path;
			let dir: string | null = null;
			if (alwaysAsk) {
				const picked = await open({ directory: true, multiple: false });
				if (typeof picked !== "string" || !picked) return;
				dir = picked;
			}
			await api.startDownload({
				url: singleLink,
				download_mode: mode,
				quality,
				format_id: null,
				playlist_items: null,
				output_dir: dir
			});
			url = "";
			analysis = null;
			busy = "idle";
			await goto("/downloads");
		} catch (e) {
			busy = "error";
			errorMessage = e instanceof Error ? e.message : String(e);
		}
	}

	async function startBatch() {
		for (const link of links) {
			try {
				await api.startDownload({
					url: link,
					download_mode: mode,
					quality,
					format_id: null,
					playlist_items: null,
					output_dir: null
				});
			} catch (e) {
				toastStore.push(e instanceof Error ? e.message : String(e), "error");
			}
		}
		url = "";
		await goto("/downloads");
	}

	async function installEngine() {
		engineInstalling = true;
		try {
			await api.engineInstall("ytdlp");
			engine = await api.engineStatus();
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		} finally {
			engineInstalling = false;
		}
	}

	const qualityOptions = $derived(
		analysis?.qualities?.length
			? [{ label: t("home.best"), height: null as number | null }].concat(
					[...new Map(
						analysis.qualities
							.filter((q) => q.height != null)
							.sort((a, b) => (b.height ?? 0) - (a.height ?? 0))
							.map((q) => [q.height, { label: `${q.height}p`, height: q.height }])
					).values()]
				)
			: [{ label: t("home.best"), height: null as number | null }]
	);
</script>

<div class="page">
	<div class="page-inner home-inner">
		<header class="hero">
			<div class="hero-mark" aria-hidden="true">
				<LogoMark size={64} />
			</div>
			<h1 class="hero-headline">{t("home.headline")}</h1>
			<p class="hero-sub">{t("home.sub")}</p>
		</header>

		{#if engine && !engine.ytdlp.found}
			<div class="engine-banner" role="alert">
				<span class="spinner" hidden={!engineInstalling}></span>
				<span>{t("home.engineMissing")}</span>
				<button class="btn btn-accent btn-sm" onclick={installEngine} disabled={engineInstalling}>
					{engineInstalling ? t("common.installing") : t("home.engineMissingCta")}
				</button>
			</div>
		{/if}

		<div class="card omnibox">
			<svg class="omnibox-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4v10" /><path d="m7.5 10.5 4.5 4.5 4.5-4.5" /><path d="M4.5 19h15" /></svg>
			<input
				class="omnibox-input"
				type="text"
				bind:value={url}
				oninput={onInput}
				placeholder={t("home.placeholder")}
				aria-label={t("home.placeholder")}
				spellcheck="false"
				autocapitalize="none"
				autocorrect="off"
			/>
			{#if busy === "analyzing"}
				<div class="spinner" role="status" aria-label={t("home.analyzing")}></div>
			{/if}
		</div>

		{#if errorMessage && busy === "error"}
			<div class="card panel error-panel" role="alert">
				<p class="panel-title">{t("home.errorTitle")}</p>
				<p class="muted mono">{errorMessage}</p>
				<div class="panel-actions">
					<button class="btn btn-sm" onclick={() => { busy = "idle"; errorMessage = ""; }}>{t("home.retry")}</button>
				</div>
			</div>
		{/if}

		{#if analysis && busy === "ready" && singleLink}
			<div class="card panel">
				{#if !analysis.supported}
					<p class="panel-title">{t("home.unsupported")}</p>
				{:else}
					<div class="preview">
						{#if analysis.thumbnail_url}
							<img class="preview-thumb" src={analysis.thumbnail_url} alt="" referrerpolicy="no-referrer" loading="lazy" onerror={(e) => ((e.currentTarget as HTMLImageElement).style.visibility = "hidden")} />
						{/if}
						<div class="preview-meta">
							<p class="preview-title">{analysis.title ?? singleLink}</p>
							<div class="preview-sub muted">
								{#if analysis.author}<span>{analysis.author}</span><span aria-hidden="true"> · </span>{/if}
								<span>{analysis.platform}</span>
								{#if analysis.duration_seconds}<span aria-hidden="true"> · </span><span>{formatDuration(analysis.duration_seconds)}</span>{/if}
							</div>
							{#if analysis.is_playlist}<p class="hint">{t("home.playlistNote")}</p>{/if}
						</div>
					</div>

					<div class="quality-row" role="group" aria-label="Quality">
						{#each qualityOptions as q (q.label)}
							<button class="pill pill-btn" class:active={quality === (q.height ? String(q.height) : "best")} onclick={() => (quality = q.height ? String(q.height) : "best")}>{q.label}</button>
						{/each}
					</div>

					<div class="mode-row">
						<div class="segmented" role="group" aria-label="Mode">
							<button aria-pressed={mode === "auto"} onclick={() => (mode = "auto")}>{t("home.modeAuto")}</button>
							<button aria-pressed={mode === "audio"} onclick={() => (mode = "audio")}>{t("home.modeAudio")}</button>
							<button aria-pressed={mode === "mute"} onclick={() => (mode = "mute")}>{t("home.modeMute")}</button>
						</div>
						<button class="btn btn-accent" onclick={startDownload}>{t("home.download")}</button>
					</div>
				{/if}
			</div>
		{/if}

		{#if links.length > 1}
			<div class="card panel">
				<div class="batch-row">
					<div>
						<p class="panel-title">{t("home.batchFound", { count: links.length })}</p>
						<p class="muted hint">{t("home.hint")}</p>
					</div>
					<button class="btn btn-accent" onclick={startBatch}>{t("home.downloadAll")}</button>
				</div>
			</div>
		{/if}

		{#if recentItems.length > 0}
			<section class="recent">
				<div class="recent-head">
					<h2>{t("home.recent")}</h2>
					<a class="recent-link" href="/downloads">{t("home.openQueue")} →</a>
				</div>
				<ul class="recent-list">
					{#each recentItems as item (item.id)}
						<li class="recent-item">
							<span class="recent-title">{item.title}</span>
							<span class="recent-status muted">{item.status === "error" ? item.error_message : item.phase ?? item.status}</span>
						</li>
					{/each}
				</ul>
			</section>
		{/if}
	</div>
</div>

<style>
	.home-inner {
		display: flex;
		flex-direction: column;
		align-items: center;
		padding-top: clamp(24px, 8vh, 72px);
		gap: 18px;
	}

	.hero {
		text-align: center;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 6px;
	}

	.hero-mark {
		width: 88px;
		height: 88px;
		border-radius: 24px;
		background: var(--spruce);
		display: grid;
		place-items: center;
		color: var(--accent);
		margin-bottom: 6px;
	}

	.hero-headline {
		font-family: var(--font-display);
		font-size: clamp(2.2rem, 6vw, 3.4rem);
		font-weight: 800;
		letter-spacing: -0.02em;
	}

	.hero-sub {
		color: var(--muted);
		max-width: 460px;
		margin: 0;
	}

	.engine-banner {
		display: flex;
		align-items: center;
		gap: 12px;
		background: var(--warn-soft);
		border: 1px solid var(--warn);
		color: var(--warn);
		padding: 10px 14px;
		border-radius: var(--r-md);
		font-size: 13px;
		max-width: 640px;
	}

	.omnibox {
		display: flex;
		align-items: center;
		gap: 14px;
		padding: 6px 18px;
		width: min(640px, 100%);
		border-radius: var(--r-pill);
	}

	.omnibox-icon {
		width: 30px;
		height: 30px;
		stroke: var(--accent);
		fill: none;
		stroke-width: 1.8;
		stroke-linecap: round;
		stroke-linejoin: round;
		flex: none;
	}

	.omnibox-input {
		flex: 1;
		border: none;
		background: transparent;
		outline: none;
		color: var(--text);
		font-size: 17px;
		padding: 14px 0;
		min-width: 0;
	}

	.omnibox-input::placeholder {
		color: var(--muted);
	}

	.panel {
		padding: 18px 20px;
		width: min(640px, 100%);
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.panel-title {
		font-weight: 650;
		margin: 0;
	}

	.panel-actions {
		display: flex;
		gap: 8px;
	}

	.error-panel {
		border-color: var(--danger);
	}

	.preview {
		display: flex;
		gap: 14px;
		align-items: center;
	}

	.preview-thumb {
		width: 132px;
		height: 88px;
		object-fit: cover;
		border-radius: var(--r-md);
		background: var(--card-2);
		flex: none;
	}

	.preview-meta {
		min-width: 0;
	}

	.preview-title {
		font-weight: 650;
		margin: 0 0 4px;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	.preview-sub {
		font-size: 12.5px;
	}

	.quality-row {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
	}

	.pill-btn {
		border: 1px solid transparent;
		cursor: pointer;
	}

	.pill-btn:hover {
		border-color: var(--border-strong);
	}

	.pill-btn.active {
		background: var(--accent-soft);
		color: var(--accent);
		border-color: var(--accent);
	}

	.mode-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		flex-wrap: wrap;
	}

	.batch-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		flex-wrap: wrap;
	}

	.recent {
		width: min(640px, 100%);
	}

	.recent-head {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		margin-bottom: 8px;
	}

	.recent-head h2 {
		font-size: 13px;
		font-family: var(--font-body);
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.07em;
		color: var(--muted);
	}

	.recent-link {
		font-size: 12.5px;
		font-weight: 600;
		color: var(--accent);
		text-decoration: none;
	}

	.recent-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.recent-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		background: var(--card);
		border: 1px solid var(--border);
		border-radius: var(--r-md);
		padding: 8px 12px;
	}

	.recent-title {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		font-size: 13px;
	}

	.recent-status {
		font-size: 11px;
		flex: none;
		max-width: 40%;
		overflow: hidden;
		text-overflow: ellipsis;
	}
</style>
