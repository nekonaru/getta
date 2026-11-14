<script lang="ts">
	import { onMount } from "svelte";
	import { convertFileSrc } from "@tauri-apps/api/core";
	import { api, type ChapterMarker, type HistoryEntry } from "#lib/api";
	import { toastStore } from "#lib/stores/toast.svelte";
	import { t } from "#lib/i18n.svelte";
	import { formatBytes, formatRelativeTime, formatDuration } from "#lib/format";

	type Kind = "all" | "video" | "audio" | "image" | "file";
	type MediaKind = Exclude<Kind, "all">;

	let entries = $state<HistoryEntry[]>([]);
	let query = $state("");
	let filter = $state<Kind>("all");
	let active = $state<HistoryEntry | null>(null);
	let playError = $state(false);
	let mediaEl = $state<HTMLMediaElement | null>(null);
	let speed = $state(1);
	let chapters = $state<ChapterMarker[]>([]);
	let chapterSelection = $state("");
	let dupGroups = $state<string[][]>([]);
	const dupIds = $derived(new Set(dupGroups.flat()));
	const dupTotal = $derived(dupGroups.reduce((sum, group) => sum + group.length - 1, 0));

	onMount(async () => {
		try {
			entries = (await api.historyList()).filter((e) => e.success && e.file_path);
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	});

	function kindOf(path: string | null): MediaKind {
		const ext = (path?.split(".").pop() ?? "").toLowerCase();
		if (["mp4", "mkv", "webm", "mov", "avi", "m4v", "ts", "flv"].includes(ext)) return "video";
		if (["mp3", "m4a", "flac", "ogg", "opus", "wav", "aac", "aiff"].includes(ext)) return "audio";
		if (["jpg", "jpeg", "png", "gif", "webp", "avif", "svg", "ico", "bmp"].includes(ext)) return "image";
		return "file";
	}

	const filtered = $derived(
		entries.filter((entry) => {
			if (filter !== "all" && kindOf(entry.file_path) !== filter) return false;
			if (query.trim()) {
				const needle = query.trim().toLowerCase();
				return (
					entry.title.toLowerCase().includes(needle) ||
					entry.platform.toLowerCase().includes(needle) ||
					(entry.file_path ?? "").toLowerCase().includes(needle)
				);
			}
			return true;
		})
	);

	const counts = $derived({
		all: entries.length,
		video: entries.filter((e) => kindOf(e.file_path) === "video").length,
		audio: entries.filter((e) => kindOf(e.file_path) === "audio").length,
		image: entries.filter((e) => kindOf(e.file_path) === "image").length,
		file: entries.filter((e) => kindOf(e.file_path) === "file").length
	});

	async function reveal(entry: HistoryEntry) {
		if (!entry.file_path) return;
		try {
			await api.revealInFolder(entry.file_path);
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function copyUrl(entry: HistoryEntry) {
		try {
			await api.setClipboardText(entry.url);
			toastStore.push(t("library.copied"));
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function removeEntry(entry: HistoryEntry) {
		if (!window.confirm(t("library.removeConfirm", { title: entry.title }))) return;
		try {
			await api.historyRemove(entry.id);
			entries = entries.filter((e) => e.id !== entry.id);
			if (active?.id === entry.id) active = null;
			toastStore.push(t("library.removed"));
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function openEntry(entry: HistoryEntry) {
		active = entry;
		playError = false;
		chapters = [];
		chapterSelection = "";
		const kind = kindOf(entry.file_path);
		if (kind === "video" || kind === "audio") {
			try {
				chapters = await api.probeChapters(entry.file_path ?? "");
			} catch {
				chapters = [];
			}
		}
	}

	function seekChapter() {
		const index = Number(chapterSelection);
		const marker = chapters[index];
		if (marker && mediaEl) mediaEl.currentTime = marker.start_seconds;
	}

	async function findDuplicates() {
		try {
			dupGroups = await api.findDuplicates();
			if (dupGroups.length === 0) {
				toastStore.push(t("library.noDuplicates"));
			} else {
				toastStore.push(t("library.duplicatesFound", { count: dupTotal }));
			}
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function removeDuplicateCopies() {
		const ids = dupGroups.flatMap((group) => group.slice(1));
		if (ids.length === 0) return;
		if (!window.confirm(t("library.removeDuplicatesConfirm", { count: ids.length }))) return;
		try {
			for (const id of ids) {
				try {
					await api.historyRemove(id);
				} catch {
					/* best effort, keep going */
				}
			}
			entries = entries.filter((entry) => !ids.includes(entry.id));
			dupGroups = [];
			if (active && ids.includes(active.id)) active = null;
			toastStore.push(t("library.duplicatesRemoved"));
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	const KIND_LABELS: Array<[Kind, () => string]> = [
		["all", () => t("library.filterAll")],
		["video", () => t("library.filterVideo")],
		["audio", () => t("library.filterAudio")],
		["image", () => t("library.filterImage")],
		["file", () => t("library.filterFile")]
	];
</script>

<div class="page">
	<div class="page-inner">
		<header class="page-head">
			<div>
				<p class="kicker">{t("library.kicker")}</p>
				<h1 class="page-title">{t("library.title")}</h1>
			</div>
			<span class="pill mono">{t("library.count", { count: entries.length })}</span>
		</header>

		{#if entries.length === 0}
			<div class="empty-state">
				<svg viewBox="0 0 256 256" width="84" height="84" aria-hidden="true"><g fill="none" stroke="var(--accent)" stroke-width="24" stroke-linecap="round" stroke-linejoin="round"><circle cx="81" cy="85" r="45"/><path d="M128 40 V152 Q128 189 91 189 Q62 189 50 163"/></g><path fill="var(--accent)" d="M88 194 Q184 190 228 92 L238 118 Q198 226 88 222 Z"/></svg>
				<h2 class="empty-title">{t("library.empty")}</h2>
				<p class="muted">{t("library.emptyHint")}</p>
				<a class="btn btn-accent" href="/">{t("library.emptyCta")}</a>
			</div>
		{:else}
			<div class="library-toolbar">
				<input class="input search" type="search" bind:value={query} placeholder={t("library.search")} aria-label={t("library.search")} spellcheck="false" />
				<button class="btn btn-sm btn-ghost" onclick={findDuplicates}>{t("library.findDuplicates")}</button>
				<div class="filters" role="group">
					{#each KIND_LABELS as [id, label] (id)}
						<button class="pill pill-btn" class:active={filter === id} onclick={() => (filter = id)}>
							{label()} ({counts[id]})
						</button>
					{/each}
				</div>
			</div>

			{#if dupGroups.length > 0}
				<div class="dup-banner" role="status">
					<span class="dup-banner-text">{t("library.duplicatesBanner", { groups: dupGroups.length, count: dupTotal })}</span>
					<div class="dup-banner-actions">
						<button class="btn btn-accent btn-sm" onclick={removeDuplicateCopies}>{t("library.removeDuplicates")}</button>
						<button class="btn btn-sm btn-ghost" onclick={() => (dupGroups = [])}>{t("library.dismiss")}</button>
					</div>
				</div>
			{/if}

			<ul class="library-grid">
				{#each filtered as entry (entry.id)}
					{@const kind = kindOf(entry.file_path)}
					<li>
						<button class="media-card" onclick={() => openEntry(entry)}>
							<span class="media-thumb {entry.thumbnail_url ? 'skeleton' : ''}" aria-hidden="true">
								{#if dupIds.has(entry.id)}<span class="dup-chip mono">{t("library.duplicate")}</span>{/if}
								{#if entry.thumbnail_url}
									<img src={entry.thumbnail_url} alt="" referrerpolicy="no-referrer" loading="lazy" onload={(e) => (e.currentTarget as HTMLImageElement).classList.add('loaded')} onerror={(e) => (e.currentTarget as HTMLImageElement).style.visibility = 'hidden'} />
								{:else}
									<span class="media-letter">{(entry.platform || "?").slice(0, 1).toUpperCase()}</span>
								{/if}
								{#if kind === "video"}
									<span class="media-play"><svg viewBox="0 0 24 24"><path d="M8 5l12 7-12 7z" /></svg></span>
								{/if}
							</span>
							<span class="media-meta">
								<span class="media-title" title={entry.title}>{entry.title}</span>
								<span class="media-sub mono">{entry.platform}{entry.file_size_bytes ? ` · ${formatBytes(entry.file_size_bytes)}` : ""}{entry.completed_at ? ` · ${formatRelativeTime(entry.completed_at)}` : ""}</span>
							</span>
						</button>
					</li>
				{/each}
			</ul>
		{/if}
	</div>
</div>

{#if active}
	<div class="veil" role="dialog" aria-modal="true" aria-label={active.title} tabindex="-1" onclick={(e) => e.target === e.currentTarget && (active = null)} onkeydown={(e) => e.key === "Escape" && (active = null)}>
		<div class="modal media-modal">
			<div class="media-modal-head">
				<p class="media-modal-title" title={active.title}>{active.title}</p>
				<button class="btn-icon" title={t("library.close")} aria-label={t("library.close")} onclick={() => (active = null)}><svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6 6 18" /></svg></button>
			</div>
			{#if playError}
				<p class="muted">{t("library.cannotPlay")}</p>
			{:else if kindOf(active.file_path) === "video"}
				<!-- svelte-ignore a11y_media_has_caption -->
				<video class="media-player" bind:this={mediaEl} src={convertFileSrc(active.file_path ?? "")} controls autoplay onloadedmetadata={() => (mediaEl && (mediaEl.playbackRate = speed))} onerror={() => (playError = true)}></video>
			{:else if kindOf(active.file_path) === "audio"}
				<audio class="media-audio" bind:this={mediaEl} src={convertFileSrc(active.file_path ?? "")} controls autoplay onloadedmetadata={() => (mediaEl && (mediaEl.playbackRate = speed))} onerror={() => (playError = true)}></audio>
			{:else if kindOf(active.file_path) === "image"}
				<img class="media-image" src={convertFileSrc(active.file_path ?? "")} alt={active.title} onerror={() => (playError = true)} />
			{:else}
				<p class="muted">{t("library.cannotPlay")}</p>
			{/if}
			{#if kindOf(active.file_path) === "video" || kindOf(active.file_path) === "audio"}
				<div class="speed-row">
					<label class="speed-label field-label" for="playback-speed">{t("library.speed")}</label>
					<select id="playback-speed" class="input speed-select" value={speed} onchange={(e) => {
						speed = Number(e.currentTarget.value);
						if (mediaEl) mediaEl.playbackRate = speed;
					}}>
						{#each [0.5, 0.75, 1, 1.25, 1.5, 1.75, 2, 2.5, 3] as s (s)}
							<option value={s}>{s}x</option>
						{/each}
					</select>
					{#if chapters.length > 0}
						<label class="speed-label field-label" for="chapters">{t("library.chapters")}</label>
						<select id="chapters" class="input chapters-select" bind:value={chapterSelection} onchange={seekChapter}>
							<option value="" disabled>{t("library.pickChapter")}</option>
							{#each chapters as marker, i (i)}
								<option value={String(i)}>{marker.title ?? t("library.chapter", { n: i + 1 })} · {formatDuration(marker.start_seconds)}</option>
							{/each}
						</select>
					{/if}
				</div>
			{/if}
			<div class="media-modal-foot">
				<span class="mono media-path">{active.file_path}</span>
				<div class="media-modal-actions">
					<button class="btn btn-sm btn-ghost" onclick={() => copyUrl(active!)}>{t("library.copyUrl")}</button>
					<button class="btn btn-sm btn-ghost danger-text" onclick={() => removeEntry(active!)}>{t("library.remove")}</button>
					<button class="btn btn-sm" onclick={() => active && reveal(active)}>{t("library.reveal")}</button>
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
	.library-toolbar {
		display: flex;
		gap: 12px;
		align-items: center;
		margin-bottom: 16px;
		flex-wrap: wrap;
	}

	.search {
		max-width: 320px;
		flex: 1;
	}

	.filters {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
	}

	.filters .pill-btn.active {
		border-color: var(--accent);
		color: var(--accent);
		background: var(--accent-soft);
	}

	.library-grid {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(158px, 1fr));
		gap: 14px;
	}

	.media-card {
		display: flex;
		flex-direction: column;
		gap: 8px;
		background: var(--card);
		border: 1px solid var(--border);
		border-radius: var(--r-md);
		padding: 10px;
		cursor: pointer;
		text-align: left;
		width: 100%;
		transition: transform 0.15s ease, border-color 0.15s ease;
		color: inherit;
	}

	.media-card:hover {
		transform: translateY(-2px);
		border-color: var(--border-strong);
	}

	.chapters-select {
		width: auto;
		min-width: 180px;
		max-width: 260px;
	}

	.dup-banner {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		background: var(--warn-soft);
		border: 1px solid var(--warn);
		color: var(--warn);
		border-radius: var(--r-sm);
		padding: 8px 12px;
		font-size: 12.5px;
		font-weight: 600;
		margin-bottom: 14px;
	}

	.dup-banner-actions {
		display: flex;
		align-items: center;
		gap: 8px;
		flex: none;
	}

	.dup-chip {
		position: absolute;
		top: 8px;
		left: 8px;
		z-index: 2;
		background: var(--warn);
		color: #fff;
		border-radius: var(--r-sm);
		padding: 2px 6px;
		font-size: 8.5px;
	}

	.media-thumb {
		position: relative;
		aspect-ratio: 1 / 0.75;
		background: var(--card-2);
		border: 1px solid var(--border-soft);
		border-radius: var(--r-sm);
		display: grid;
		place-items: center;
		overflow: hidden;
	}

	.media-thumb img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		opacity: 0;
		transition: opacity 0.35s ease;
	}

	:global(.media-thumb img.loaded) {
		opacity: 1;
	}

	.media-letter {
		font-family: var(--font-display);
		font-size: 2rem;
		font-weight: 800;
		color: var(--muted);
	}

	.media-play {
		position: absolute;
		inset: 0;
		display: grid;
		place-items: center;
		background: rgba(16, 18, 20, 0.35);
		opacity: 0;
		transition: opacity 0.15s ease;
	}

	.media-card:hover .media-play,
	.media-card:focus-visible .media-play {
		opacity: 1;
	}

	.media-play svg {
		width: 34px;
		height: 34px;
		fill: #fff;
		stroke: none;
	}

	.media-meta {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}

	.media-title {
		font-size: 12px;
		font-weight: 600;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.media-sub {
		font-size: 9px;
		color: var(--muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.media-modal {
		width: min(880px, 96vw);
	}

	.media-modal-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		margin-bottom: 12px;
	}

	.media-modal-title {
		margin: 0;
		font-weight: 650;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.media-player {
		width: 100%;
		max-height: 62vh;
		background: #000;
		border-radius: var(--r-sm);
	}

	.media-audio {
		width: 100%;
	}

	.media-image {
		max-width: 100%;
		max-height: 62vh;
		object-fit: contain;
		display: block;
		margin: 0 auto;
		border-radius: var(--r-sm);
		background: var(--card-2);
	}

	.speed-row {
		display: flex;
		align-items: center;
		gap: 10px;
		margin-top: 12px;
	}

	.speed-label {
		margin: 0;
	}

	.speed-select {
		width: auto;
		min-width: 88px;
	}

	.media-modal-foot {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		margin-top: 12px;
		border-top: 1px solid var(--border);
		padding-top: 12px;
		flex-wrap: wrap;
	}

	.media-modal-actions {
		display: flex;
		align-items: center;
		gap: 8px;
		flex-shrink: 0;
	}

	.danger-text {
		color: var(--bad);
	}

	.media-path {
		font-size: 10px;
		color: var(--muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
</style>
