<script lang="ts">
	import { onMount } from "svelte";
	import { goto } from "$app/navigation";
	import { api, type Analysis, type EngineStatus } from "#lib/api";
	import { downloadsStore, readStats } from "#lib/stores/downloads.svelte";
	import { toastStore } from "#lib/stores/toast.svelte";
	import { settingsStore } from "#lib/stores/settings.svelte";
	import { t } from "#lib/i18n.svelte";
	import { formatBytes, formatDuration } from "#lib/format";
	import LogoMark from "#lib/components/LogoMark.svelte";

	let url = $state("");
	let busy = $state<"idle" | "analyzing" | "ready" | "starting" | "error">("idle");
	let analysis = $state<Analysis | null>(null);
	let errorMessage = $state("");
	let engine = $state<EngineStatus | null>(null);
	let engineInstalling = $state(false);
	let quality = $state("best");
	let mode = $state<"auto" | "audio" | "mute">("auto");
	let playlistSelected = $state<Set<number>>(new Set());
	let analyzeTimer: ReturnType<typeof setTimeout> | undefined;

	const links = $derived(parseLinks(url));
	const singleLink = $derived(links.length === 1 ? links[0] : null);
	const searchQuery = $derived(
		url
			.trim()
			.split(/\s+/)
			.filter(Boolean)
			.join(" ")
	);
	const isSearch = $derived(!looksLikeUrl(url) && searchQuery.length >= 3);
	const downloadTarget = $derived(singleLink ?? (isSearch ? `ytsearch5:${searchQuery}` : null));

	const recentItems = $derived(
		[...downloadsStore.items].sort((a, b) => b.id - a.id).slice(0, 4)
	);

	const stats = readStats();

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
			.map((token) => token.trim())
			.filter((token) => /^(https?:\/\/|magnet:|p2p:)/.test(token));
		return [...new Set(tokens)].slice(0, 200);
	}

	function looksLikeUrl(text: string): boolean {
		return /^(https?:\/\/|magnet:|p2p:).+/.test(text.trim());
	}

	function onInput() {
		clearTimeout(analyzeTimer);
		analysis = null;
		if (!looksLikeUrl(url) && !isSearch) {
			busy = "idle";
			errorMessage = "";
			return;
		}
		busy = "analyzing";
		errorMessage = "";
		analyzeTimer = setTimeout(() => void analyze(), 550);
	}

	async function analyze() {
		const target = downloadTarget;
		if (!target) return;
		try {
			const result = await api.analyzeUrl(target);
			analysis = result;
			busy = "ready";
			quality = "best";
			playlistSelected = new Set(result.playlist_entries.map((e) => e.index));
		} catch (e) {
			busy = "error";
			errorMessage = e instanceof Error ? e.message : String(e);
		}
	}

	async function startDownload(playlistItems?: string) {
		const target = downloadTarget;
		if (!target) return;
		busy = "starting";
		try {
			const alwaysAsk = settingsStore.value?.always_ask_path;
			let dir: string | null = null;
			if (alwaysAsk) {
				const { open } = await import("@tauri-apps/plugin-dialog");
				const picked = await open({ directory: true, multiple: false });
				if (typeof picked !== "string" || !picked) {
					busy = "ready";
					return;
				}
				dir = picked;
			}
			await api.startDownload({
				url: target,
				download_mode: mode,
				quality,
				format_id: null,
				playlist_items: playlistItems ?? null,
				output_dir: dir
			});
			url = "";
			analysis = null;
			busy = "idle";
			await goto("/queue");
		} catch (e) {
			busy = "error";
			errorMessage = e instanceof Error ? e.message : String(e);
			busy = "ready";
		}
	}

	async function startPlaylistDownload() {
		const indices = [...playlistSelected].sort((a, b) => a - b);
		if (indices.length === 0) return;
		const ranges: string[] = [];
		let rangeStart = indices[0];
		let previous = indices[0];
		for (let i = 1; i <= indices.length; i++) {
			const current = indices[i];
			if (current !== previous + 1) {
				ranges.push(rangeStart === previous ? `${rangeStart}` : `${rangeStart}-${previous}`);
				rangeStart = current;
			}
			previous = current;
		}
		await startDownload(ranges.join(","));
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
		await goto("/queue");
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

	const isPlaylist = $derived(analysis?.is_playlist === true && (analysis?.playlist_entries?.length ?? 0) > 0);
</script>

<div class="page">
	<div class="page-inner home-inner">
		<div class="home-left">
			<p class="kicker">{t("home.kicker")}</p>
			<h1 class="headline">
				<span class="headline-line">{t("home.headline1")}</span>
				<span class="headline-line headline-accent">{t("home.headline2")}</span>
			</h1>
			<p class="sub">{t("home.sub")}</p>

			{#if engine && !engine.ytdlp.found}
				<div class="engine-banner" role="alert">
					<span>{t("home.engineMissing")}</span>
					<button class="btn btn-accent btn-sm" onclick={installEngine} disabled={engineInstalling}>
						{engineInstalling ? t("common.installing") : t("home.engineMissingCta")}
					</button>
				</div>
			{/if}

			<div class="getup card">
				<LogoMark size={30} />
				<input
					class="getup-input"
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
				<button class="btn btn-accent fetch-btn" onclick={() => startDownload()} disabled={busy === "analyzing" || busy === "starting" || !downloadTarget}>
					{busy === "starting" ? t("home.fetching") : t("home.button")}
				</button>
			</div>

			<p class="hint hotkey-hint">
				<kbd class="kbd">Ctrl</kbd>+<kbd class="kbd">Shift</kbd>+<kbd class="kbd">D</kbd>
				{t("home.hotkeyHint")}
			</p>

			{#if busy === "error"}
				<div class="card panel error-panel" role="alert">
					<p class="panel-title">{t("home.errorTitle")}</p>
					<p class="muted mono">{errorMessage}</p>
					<button class="btn btn-sm" onclick={() => { busy = "idle"; errorMessage = ""; }}>{t("home.retry")}</button>
				</div>
			{/if}

			{#if analysis && busy === "ready" && analysis.supported && downloadTarget}
				<div class="card panel">
					<div class="preview">
						{#if analysis.thumbnail_url}
							<img class="preview-thumb" src={analysis.thumbnail_url} alt="" referrerpolicy="no-referrer" loading="lazy" onerror={(e) => ((e.currentTarget as HTMLImageElement).style.visibility = "hidden")} />
						{/if}
						<div class="preview-meta">
							<p class="preview-title">{analysis.title ?? downloadTarget}</p>
							<div class="preview-sub muted">
								<span class="mono">{analysis.platform}</span>
								{#if analysis.duration_seconds}<span>· {formatDuration(analysis.duration_seconds)}</span>{/if}
								{#if analysis.author}<span>· {analysis.author}</span>{/if}
							</div>
						</div>
					</div>

					{#if isPlaylist}
						<div class="playlist-block">
							<div class="playlist-head">
								<span class="field-label" style="margin:0">{isSearch ? t("home.searchNote") : t("home.playlistNote")}</span>
								<span class="playlist-actions">
									<button class="btn btn-sm btn-ghost" onclick={() => (playlistSelected = new Set(analysis?.playlist_entries.map((e) => e.index) ?? []))}>{t("home.playlistSelectAll")}</button>
									<button class="btn btn-sm btn-ghost" onclick={() => (playlistSelected = new Set())}>{t("home.playlistSelectNone")}</button>
								</span>
							</div>
							<ul class="playlist-list">
								{#each analysis.playlist_entries as entry (entry.index)}
									<li>
										<label class="playlist-entry">
											<input type="checkbox" checked={playlistSelected.has(entry.index)} onchange={() => {
												const next = new Set(playlistSelected);
												if (next.has(entry.index)) next.delete(entry.index);
												else next.add(entry.index);
												playlistSelected = next;
											}} />
											<span class="playlist-index mono">{entry.index}</span>
											<span class="playlist-entry-title">{entry.title}</span>
										</label>
									</li>
								{/each}
							</ul>
							<div class="mode-row">
								<div class="segmented" role="group" aria-label="Mode">
									<button aria-pressed={mode === "auto"} onclick={() => (mode = "auto")}>{t("home.modeAuto")}</button>
									<button aria-pressed={mode === "audio"} onclick={() => (mode = "audio")}>{t("home.modeAudio")}</button>
									<button aria-pressed={mode === "mute"} onclick={() => (mode = "mute")}>{t("home.modeMute")}</button>
								</div>
								<button class="btn btn-accent" onclick={startPlaylistDownload} disabled={playlistSelected.size === 0}>
									{t("home.playlistDownload", { count: playlistSelected.size })}
								</button>
							</div>
						</div>
					{:else}
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
							<button class="btn btn-accent" onclick={() => startDownload()}>{t("home.download")}</button>
						</div>
					{/if}
				</div>
			{/if}

			{#if analysis && busy === "ready" && !analysis.supported}
				<div class="card panel">
					<p class="panel-title">{t("home.unsupported")}</p>
				</div>
			{/if}

			{#if links.length > 1}
				<div class="card panel">
					<div class="batch-row">
						<div>
							<p class="panel-title">{t("home.batchFound", { count: links.length })}</p>
							<p class="muted hint">{t("home.hint")}</p>
						</div>
						<button class="btn btn-accent" onclick={startBatch}>{t("home.downloadAll", { count: links.length })}</button>
					</div>
				</div>
			{/if}
		</div>

		<aside class="home-right">
			<div class="cargo card">
				<div class="cargo-head">
					<h2 class="cargo-title">{t("home.recent")}</h2>
					<a class="cargo-link" href="/queue">{t("home.openQueue")} →</a>
				</div>
				{#if recentItems.length === 0}
					<p class="muted cargo-empty">{t("queue.emptyHint")}</p>
				{:else}
					<ul class="cargo-list">
						{#each recentItems as item (item.id)}
							<li class="cargo-item">
								<span class="cargo-index mono">{item.status === "active" ? "→" : item.status === "done" ? "✓" : "·"}</span>
								<span class="cargo-body">
									<span class="cargo-title-text">{item.title}</span>
									<span class="cargo-status mono">{item.status === "active" ? t("queue.statusActive") : item.status === "done" ? t("queue.statusDone") : item.status}</span>
								</span>
								{#if item.status === "active"}
									<span class="cargo-pct mono">{Math.floor(item.percent)}%</span>
								{/if}
							</li>
						{/each}
					</ul>
				{/if}
			</div>

			<p class="stats-line mono" aria-live="polite">
				{t("home.statsLine", { count: stats.downloads, bytes: formatBytes(stats.bytes) })}
			</p>
		</aside>
	</div>
</div>

<style>
	.home-inner {
		display: grid;
		grid-template-columns: 1.25fr 0.75fr;
		gap: 28px;
		align-items: start;
		padding-top: clamp(8px, 5vh, 40px);
	}

	.kicker {
		margin-bottom: 18px;
	}

	.headline {
		font-family: var(--font-display);
		font-weight: 800;
		letter-spacing: -0.03em;
		font-size: clamp(3rem, 8.2vw, 4.6rem);
		line-height: 0.98;
		display: flex;
		flex-direction: column;
	}

	.headline-line {
		display: block;
	}

	.home-left {
		min-width: 0;
	}

	.headline-accent {
		font-style: italic;
		color: var(--accent);
	}

	.sub {
		color: var(--muted);
		max-width: 52ch;
		margin: 18px 0 22px;
		font-size: 14.5px;
	}

	.engine-banner {
		display: flex;
		align-items: center;
		gap: 12px;
		background: var(--warn-soft);
		border: 1px solid var(--warn);
		color: var(--warn);
		border-radius: var(--r-sm);
		padding: 10px 14px;
		font-size: 12.5px;
		font-weight: 600;
		margin-bottom: 12px;
		justify-content: space-between;
	}

	.getup {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 6px 8px 6px 18px;
		border-radius: var(--r-sm);
	}

	.getup :global(svg) {
		color: var(--accent);
		flex: none;
	}

	.getup-input {
		flex: 1;
		border: none;
		background: transparent;
		outline: none;
		color: var(--text);
		font-size: 16.5px;
		padding: 14px 0;
		min-width: 0;
		font-family: var(--font-body);
	}

	.getup-input::placeholder {
		color: var(--muted);
	}

	.fetch-btn {
		flex: none;
		padding: 12px 22px;
		font-size: 13px;
	}

	.hotkey-hint {
		margin-top: 10px;
	}

	.panel {
		padding: 16px 18px;
		margin-top: 14px;
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.panel-title {
		font-weight: 650;
		margin: 0;
	}

	.error-panel {
		border-color: var(--bad);
	}

	.preview {
		display: flex;
		gap: 14px;
		align-items: center;
	}

	.preview-thumb {
		width: 148px;
		height: 92px;
		object-fit: cover;
		border-radius: var(--r-sm);
		background: var(--card-2);
		flex: none;
		border: 1px solid var(--border-soft);
	}

	.preview-meta {
		min-width: 0;
	}

	.preview-title {
		margin: 0 0 4px;
		font-weight: 650;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	.preview-sub {
		font-size: 11.5px;
		display: flex;
		gap: 4px;
		flex-wrap: wrap;
	}

	.quality-row {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
	}

	.playlist-block {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.playlist-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		flex-wrap: wrap;
	}

	.playlist-actions {
		display: flex;
		gap: 6px;
	}

	.playlist-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		max-height: 220px;
		overflow-y: auto;
		border: 1px solid var(--border);
		border-radius: var(--r-sm);
	}

	.playlist-entry {
		display: grid;
		grid-template-columns: 18px 34px 1fr;
		gap: 10px;
		align-items: center;
		padding: 8px 12px;
		font-size: 12px;
		cursor: pointer;
	}

	.playlist-entry:hover {
		background: var(--card-2);
	}

	.playlist-index {
		color: var(--muted);
	}

	.playlist-entry-title {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
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

	.home-right {
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.cargo {
		padding: 16px 18px;
	}

	.cargo-head {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		margin-bottom: 12px;
		border-bottom: 2px solid var(--border);
		padding-bottom: 10px;
	}

	.cargo-title {
		font-family: var(--font-mono);
		font-size: 10.5px;
		letter-spacing: 0.2em;
		text-transform: uppercase;
		color: var(--text);
		font-weight: 500;
	}

	.cargo-link {
		font-family: var(--font-mono);
		font-size: 10.5px;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--accent);
		text-decoration: none;
	}

	.cargo-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
	}

	.cargo-item {
		display: grid;
		grid-template-columns: 22px 1fr auto;
		gap: 10px;
		align-items: center;
		padding: 9px 0;
		border-bottom: 1px solid var(--border-soft);
	}

	.cargo-item:last-child {
		border-bottom: none;
	}

	.cargo-index {
		color: var(--accent);
		font-size: 13px;
	}

	.cargo-body {
		min-width: 0;
		display: flex;
		flex-direction: column;
	}

	.cargo-title-text {
		font-size: 12px;
		font-weight: 600;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.cargo-status {
		font-size: 9px;
		letter-spacing: 0.14em;
		text-transform: uppercase;
		color: var(--muted);
		margin-top: 2px;
	}

	.cargo-pct {
		font-size: 12px;
		color: var(--accent);
	}

	.cargo-empty {
		font-size: 12px;
		margin: 8px 0 0;
	}

	.stats-line {
		font-size: 10px;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--muted);
		padding: 0 2px;
	}

	@media (max-width: 960px) {
		.home-inner {
			grid-template-columns: 1fr;
		}

		.headline {
			font-size: clamp(2.6rem, 13vw, 4rem);
		}
	}
</style>
