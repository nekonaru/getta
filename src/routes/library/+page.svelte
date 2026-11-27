<script lang="ts">
	import { onMount } from "svelte";
	import { convertFileSrc } from "@tauri-apps/api/core";
	import { goto } from "$app/navigation";
	import { api, type ChapterMarker, type HistoryEntry } from "#lib/api";
	import { toastStore } from "#lib/stores/toast.svelte";
	import { t } from "#lib/i18n.svelte";
	import { formatBytes, formatRelativeTime, formatDuration } from "#lib/format";

	type Kind = "all" | "video" | "audio" | "image" | "file";
	type MediaKind = Exclude<Kind, "all">;

	type SortKey = "newest" | "oldest" | "name" | "size";
	let entries = $state<HistoryEntry[]>([]);
	let query = $state("");
	let filter = $state<Kind>("all");
	let sort = $state<SortKey>("newest");
	let selectionMode = $state(false);
	let selected = $state<Set<string>>(new Set());
	let active = $state<HistoryEntry | null>(null);
	let playError = $state(false);
	let mediaEl = $state<HTMLMediaElement | null>(null);
	let speed = $state(1);
	let chapters = $state<ChapterMarker[]>([]);
	let chapterSelection = $state("");
	let dupGroups = $state<string[][]>([]);
	let showStats = $state(false);
	let renameOpen = $state(false);
	let renameTemplate = $state("{title}");
	let undoEntries = $state<HistoryEntry[]>([]);
	let dialogEl = $state<HTMLDivElement | null>(null);
	let lastFocused: HTMLElement | null = null;

	$effect(() => {
		if (active) {
			lastFocused = document.activeElement as HTMLElement | null;
			queueMicrotask(() => dialogEl?.focus());
		} else if (lastFocused) {
			queueMicrotask(() => lastFocused?.focus());
			lastFocused = null;
		}
	});

	function trapFocus(event: KeyboardEvent) {
		if (!active || event.key !== "Tab" || !dialogEl) return;
		const focusable = dialogEl.querySelectorAll<HTMLElement>(
			'button, a[href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
		);
		if (focusable.length === 0) return;
		const first = focusable[0];
		const last = focusable[focusable.length - 1];
		if (event.shiftKey && document.activeElement === first) {
			event.preventDefault();
			last.focus();
		} else if (!event.shiftKey && document.activeElement === last) {
			event.preventDefault();
			first.focus();
		}
	}
	// The first entry of each group is the one we keep, so only extra
	// copies carry the duplicate badge.
	const dupIds = $derived(new Set(dupGroups.flatMap((group) => group.slice(1))));
	const dupTotal = $derived(dupGroups.reduce((sum, group) => sum + group.length - 1, 0));

	const storageStats = $derived.by(() => {
		const byPlatform = new Map<string, { count: number; bytes: number }>();
		for (const entry of entries) {
			const row = byPlatform.get(entry.platform) ?? { count: 0, bytes: 0 };
			row.count += 1;
			row.bytes += entry.file_size_bytes ?? 0;
			byPlatform.set(entry.platform, row);
		}
		const totalBytes = entries.reduce((sum, e) => sum + (e.file_size_bytes ?? 0), 0);
		return { byPlatform: [...byPlatform.entries()], totalBytes };
	});

	async function patchEntry(id: string, patch: Parameters<typeof api.historyPatch>[1]) {
		try {
			const ok = await api.historyPatch(id, patch);
			if (ok) {
				entries = entries.map((e) => (e.id === id ? { ...e, ...patch, tags: patch.tags ?? e.tags, rating: patch.rating ?? e.rating, note: patch.note ?? e.note } : e));
				// Mutate in place so the focus effect does not re-run and move
				// focus away from the control being edited.
				if (active?.id === id) {
					if (patch.tags !== undefined) active.tags = patch.tags;
					if (patch.rating !== undefined) active.rating = patch.rating;
					if (patch.note !== undefined) active.note = patch.note;
					if (patch.favorite !== undefined) active.favorite = patch.favorite;
					if (patch.watch_later !== undefined) active.watch_later = patch.watch_later;
				}
			}
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function doBatchRename() {
		try {
			const ids = [...selected];
			const count = await api.batchRename(ids, renameTemplate);
			toastStore.push(t("library.renamed", { count }), "info");
			renameOpen = false;
			entries = await api.historyList();
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function doExportHtml() {
		try {
			const { save } = await import("@tauri-apps/plugin-dialog");
			const path = await save({ filters: [{ name: "HTML", extensions: ["html"] }], defaultPath: "getta-library.html" });
			if (!path) return;
			await api.exportLibraryHtml(path);
			toastStore.push(t("library.htmlExported"), "info");
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

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

	const sorted = $derived.by(() => {
		const list = [...filtered];
		switch (sort) {
			case "oldest":
				list.sort((a, b) => a.completed_at - b.completed_at);
				break;
			case "name":
				list.sort((a, b) => a.title.localeCompare(b.title));
				break;
			case "size":
				list.sort((a, b) => (b.file_size_bytes ?? 0) - (a.file_size_bytes ?? 0));
				break;
			default:
				list.sort((a, b) => b.completed_at - a.completed_at);
		}
		return list;
	});

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

	async function reDownload(entry: HistoryEntry) {
		try {
			await api.startDownload({
				url: entry.url,
				download_mode: (entry.download_mode as "auto" | "audio" | "mute" | "subs" | null) ?? "auto",
				quality: entry.quality ?? "best",
				format_id: null,
				playlist_items: null,
				output_dir: null
			});
			toastStore.push(t("common.queued"), "info");
			active = null;
			await goto("/queue");
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
		const id = entry.id;
		if (kind === "video" || kind === "audio") {
			try {
				const markers = await api.probeChapters(entry.file_path ?? "");
				if (active?.id === id) chapters = markers;
			} catch {
				if (active?.id === id) chapters = [];
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

	function toggleSelect(id: string) {
		const next = new Set(selected);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		selected = next;
	}

	function enterSelectionMode() {
		selectionMode = true;
		selected = new Set();
	}

	function cancelSelection() {
		selectionMode = false;
		selected = new Set();
	}

	async function removeSelected() {
		const ids = [...selected];
		if (ids.length === 0) return;
		if (!window.confirm(t("library.removeSelectedConfirm", { count: ids.length }))) return;
		undoEntries = entries.filter((entry) => ids.includes(entry.id));
		for (const id of ids) {
			try {
				await api.historyRemove(id);
			} catch {
				/* best effort, keep going */
			}
		}
		entries = entries.filter((entry) => !selected.has(entry.id));
		if (active && selected.has(active.id)) active = null;
		toastStore.push(t("library.removedSelection", { count: ids.length }));
		cancelSelection();
	}

	async function undoRemove() {
		const rows = undoEntries;
		if (rows.length === 0) return;
		try {
			for (const entry of rows) await api.historyRestore(entry);
			entries = await api.historyList();
			undoEntries = [];
			toastStore.push(t("library.undoDone", { count: rows.length }), "info");
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function deleteDuplicateFiles() {
		const ids = dupGroups.flatMap((group) => group.slice(1));
		const paths = ids
			.map((id) => entries.find((entry) => entry.id === id)?.file_path)
			.filter((path): path is string => !!path);
		if (paths.length === 0) return;
		if (!window.confirm(t("library.deleteDuplicatesConfirm", { count: paths.length }))) return;
		try {
			const removed = new Set(await api.deleteFiles(paths));
			const removedIds = ids.filter((id) => {
				const path = entries.find((entry) => entry.id === id)?.file_path;
				return path != null && removed.has(path);
			});
			for (const id of removedIds) {
				try {
					await api.historyRemove(id);
				} catch {
					/* best effort */
				}
			}
			entries = entries.filter((entry) => !removedIds.includes(entry.id));
			if (active && removedIds.includes(active.id)) active = null;
			dupGroups = [];
			toastStore.push(t("library.duplicatesDeleted", { count: removedIds.length }));
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	const relativeLabels = () => ({
		justNow: t("time.justNow"),
		minutesAgo: t("time.minutesAgo"),
		hoursAgo: t("time.hoursAgo"),
		daysAgo: t("time.daysAgo")
	});

	const KIND_LABELS: Array<[Kind, () => string]> = [
		["all", () => t("library.filterAll")],
		["video", () => t("library.filterVideo")],
		["audio", () => t("library.filterAudio")],
		["image", () => t("library.filterImage")],
		["file", () => t("library.filterFile")]
	];
</script>

<svelte:window onkeydown={(e) => { if (e.key === "Escape" && active) active = null; }} />

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
				<label class="sort-label" for="sort">{t("library.sort")}</label>
				<select id="sort" class="input sort-select" bind:value={sort} aria-label={t("library.sort")}>
					<option value="newest">{t("library.sortNewest")}</option>
					<option value="oldest">{t("library.sortOldest")}</option>
					<option value="name">{t("library.sortName")}</option>
					<option value="size">{t("library.sortSize")}</option>
				</select>
				<button class="btn btn-sm btn-ghost" onclick={() => (showStats = !showStats)}>{t("library.storage")}</button>
				<button class="btn btn-sm btn-ghost" onclick={doExportHtml}>{t("library.exportHtml")}</button>
				<button class="btn btn-sm btn-ghost" onclick={findDuplicates}>{t("library.findDuplicates")}</button>
				<button class="btn btn-sm btn-ghost" onclick={enterSelectionMode}>{t("library.select")}</button>
				<div class="filters" role="group">
					{#each KIND_LABELS as [id, label] (id)}
						<button class="pill pill-btn" class:active={filter === id} onclick={() => (filter = id)}>
							{label()} ({counts[id]})
						</button>
					{/each}
				</div>
			</div>

			{#if showStats}
				<div class="card stats-panel">
					<p class="stats-total">{t("library.totalSize")}: <span class="mono">{formatBytes(storageStats.totalBytes)}</span></p>
					{#each storageStats.byPlatform as [platform, row] (platform)}
						<div class="stats-row">
							<span class="mono">{platform}</span>
							<span class="mono">{formatBytes(row.bytes)}</span>
							<span class="mono">{row.count}</span>
						</div>
					{/each}
				</div>
			{/if}

			{#if undoEntries.length > 0}
				<div class="card undo-bar" role="status">
					<span>{t("library.undoHint", { count: undoEntries.length })}</span>
					<button class="btn btn-sm" onclick={undoRemove}>{t("library.undo")}</button>
					<button class="btn btn-sm btn-ghost" onclick={() => (undoEntries = [])}>{t("library.dismiss")}</button>
				</div>
			{/if}

			{#if renameOpen}
				<div class="card rename-panel">
					<input class="input" bind:value={renameTemplate} aria-label={t("library.renameTemplate")} />
					<button class="btn btn-accent btn-sm" onclick={doBatchRename}>{t("library.applyRename")}</button>
					<button class="btn btn-sm btn-ghost" onclick={() => (renameOpen = false)}>{t("library.cancel")}</button>
				</div>
			{/if}

			{#if selectionMode}
				<div class="selection-bar" role="status">
					<span class="selection-count mono">{selected.size} {t("library.selected")}</span>
					<div class="selection-actions">
						<button class="btn btn-sm" onclick={() => (renameOpen = true)}>{t("library.batchRename")}</button>
						<button class="btn btn-accent btn-sm" onclick={removeSelected}>{t("library.removeSelected", { count: selected.size })}</button>
						<button class="btn btn-sm btn-ghost" onclick={cancelSelection}>{t("library.cancel")}</button>
					</div>
				</div>
			{/if}

			{#if dupGroups.length > 0}
				<div class="dup-banner" role="status">
					<span class="dup-banner-text">{t("library.duplicatesBanner", { groups: dupGroups.length, count: dupTotal })}</span>
					<div class="dup-banner-actions">
						<button class="btn btn-sm" onclick={removeDuplicateCopies}>{t("library.removeDuplicates")}</button>
						<button class="btn btn-sm" onclick={deleteDuplicateFiles}>{t("library.deleteDuplicates")}</button>
						<button class="btn btn-sm btn-ghost" onclick={() => (dupGroups = [])}>{t("library.dismiss")}</button>
					</div>
				</div>
			{/if}

			<ul class="library-grid">
				{#each sorted as entry (entry.id)}
					{@const kind = kindOf(entry.file_path)}
					{@const isSelected = selected.has(entry.id)}
					<li>
						<button class="media-card" class:selected={isSelected} onclick={() => selectionMode ? toggleSelect(entry.id) : openEntry(entry)}>
							<span class="media-thumb {entry.thumbnail_url ? 'skeleton' : ''}" aria-hidden="true">
								{#if dupIds.has(entry.id)}<span class="dup-chip mono">{t("library.duplicate")}</span>{/if}
								{#if selectionMode}<span class="select-check mono" aria-hidden="true">{isSelected ? "✓" : ""}</span>{/if}
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
								<span class="media-sub mono">{entry.platform}{entry.file_size_bytes ? ` · ${formatBytes(entry.file_size_bytes)}` : ""}{entry.completed_at ? ` · ${formatRelativeTime(entry.completed_at, relativeLabels())}` : ""}</span>
							</span>
						</button>
					</li>
				{/each}
			</ul>
		{/if}
	</div>
</div>

{#if active}
	<div class="veil" bind:this={dialogEl} role="dialog" aria-modal="true" aria-label={active.title} tabindex="-1" onclick={(e) => e.target === e.currentTarget && (active = null)} onkeydown={(e) => { trapFocus(e); if (e.key === "Escape") active = null; }}>
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
			<div class="card meta-panel">
				<label class="meta-field">
					<span>{t("library.tags")}</span>
					<input class="input" value={(active.tags ?? []).join(", ")} aria-label={t("library.tags")} onchange={(e) => patchEntry(active!.id, { tags: e.currentTarget.value.split(",").map((s) => s.trim()) })} />
				</label>
				<label class="meta-field">
					<span>{t("library.rating")}</span>
					<select class="input" value={active.rating ?? 0} aria-label={t("library.rating")} onchange={(e) => patchEntry(active!.id, { rating: Number(e.currentTarget.value) })}>
						{#each [0, 1, 2, 3, 4, 5] as n (n)}<option value={n}>{n === 0 ? t("library.unrated") : `${n} / 5`}</option>{/each}
					</select>
				</label>
				<label class="meta-field">
					<span>{t("library.note")}</span>
					<textarea class="input" rows="2" value={active.note ?? ""} aria-label={t("library.note")} onchange={(e) => patchEntry(active!.id, { note: e.currentTarget.value })}></textarea>
				</label>
				<div class="meta-toggles">
					<button class="btn btn-sm" onclick={() => patchEntry(active!.id, { favorite: !active!.favorite })}>{active.favorite ? t("library.unfavorite") : t("library.favorite")}</button>
					<button class="btn btn-sm" onclick={() => patchEntry(active!.id, { watch_later: !active!.watch_later })}>{active.watch_later ? t("library.removeWatchLater") : t("library.watchLater")}</button>
				</div>
			</div>
			<div class="media-modal-foot">
				<div class="media-foot-left">
					<span class="mono media-path">{active.file_path}</span>
					<div class="media-facts">
						{#if active.file_size_bytes}<span class="pill mono">{formatBytes(active.file_size_bytes)}</span>{/if}
						<span class="pill mono" title={new Date(active.completed_at * 1000).toISOString()}>{formatRelativeTime(active.completed_at, relativeLabels())}</span>
					</div>
				</div>
				<div class="media-modal-actions">
					<button class="btn btn-sm btn-ghost" onclick={() => reDownload(active!)}>{t("queue.redownload")}</button>
					<button class="btn btn-sm btn-ghost" onclick={() => copyUrl(active!)}>{t("library.copyUrl")}</button>
					<button class="btn btn-sm btn-ghost danger-text" onclick={() => removeEntry(active!)}>{t("library.remove")}</button>
					<button class="btn btn-sm" onclick={() => active && reveal(active)}>{t("library.reveal")}</button>
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
	.undo-bar {
		padding: 12px;
		display: flex;
		align-items: center;
		gap: 10px;
		margin: 12px 0;
	}

	.stats-panel,
	.rename-panel,
	.meta-panel {
		padding: 14px;
		margin: 12px 0;
		display: flex;
		flex-wrap: wrap;
		gap: 10px;
		align-items: center;
	}

	.stats-row {
		display: flex;
		gap: 12px;
		min-width: 260px;
	}

	.meta-field {
		display: flex;
		flex-direction: column;
		gap: 4px;
		min-width: 180px;
	}

	.meta-toggles {
		display: flex;
		gap: 8px;
	}

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

	.sort-label {
		font-size: 11px;
		font-weight: 700;
		color: var(--muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.sort-select {
		width: auto;
		min-width: 140px;
	}

	.selection-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		background: var(--accent-soft);
		border: 1px solid var(--accent);
		border-radius: var(--r-sm);
		padding: 8px 12px;
		margin-bottom: 12px;
	}

	.selection-count {
		font-size: 12px;
		font-weight: 700;
	}

	.selection-actions {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.media-card.selected {
		border-color: var(--accent);
		box-shadow: 0 0 0 2px var(--accent) inset;
	}

	.select-check {
		position: absolute;
		top: 8px;
		right: 8px;
		z-index: 2;
		width: 22px;
		height: 22px;
		border-radius: 50%;
		background: var(--accent);
		color: var(--on-accent);
		display: grid;
		place-items: center;
		font-size: 12px;
		font-weight: 800;
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

	.media-foot-left {
		display: flex;
		flex-direction: column;
		gap: 6px;
		min-width: 0;
	}

	.media-facts {
		display: flex;
		gap: 6px;
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
