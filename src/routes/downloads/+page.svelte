<script lang="ts">
	import { goto } from "$app/navigation";
	import { api, type HistoryEntry, type QueueItemInfo } from "#lib/api";
	import { downloadsStore } from "#lib/stores/downloads.svelte";
	import { settingsStore } from "#lib/stores/settings.svelte";
	import { toastStore } from "#lib/stores/toast.svelte";
	import { t } from "#lib/i18n";
	import { formatBytes, formatEta, formatRelativeTime, formatSpeed } from "#lib/format";

	type Filter = "all" | "active" | "queued" | "done" | "failed";
	let tab = $state<"queue" | "history">("queue");
	let filter = $state<Filter>("all");
	const FILTERS: Array<{ id: Filter }> = [
		{ id: "all" },
		{ id: "active" },
		{ id: "queued" },
		{ id: "done" },
		{ id: "failed" }
	];

	const items = $derived(downloadsStore.sorted(["active", "paused", "queued", "done", "error"]));
	const filtered = $derived(
		items.filter((i) => {
			switch (filter) {
				case "all":
					return true;
				case "active":
					return i.status === "active" || i.status === "paused";
				case "queued":
					return i.status === "queued";
				case "done":
					return i.status === "done";
				case "failed":
					return i.status === "error";
			}
		})
	);
	const counts = $derived({
		active: items.filter((i) => i.status === "active" || i.status === "paused").length,
		queued: items.filter((i) => i.status === "queued").length,
		failed: items.filter((i) => i.status === "error").length
	});
	const history = $derived(downloadsStore.history);

	async function reveal(path: string | null) {
		if (!path) return;
		try {
			await api.revealInFolder(path);
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function removeItem(item: QueueItemInfo, deleteFile: boolean) {
		if (deleteFile && !window.confirm(t("downloads.confirmRemoveFile", { title: item.title }))) return;
		try {
			await api.removeDownload(item.id, deleteFile);
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function clearFinished() {
		if (!window.confirm(t("downloads.confirmClear"))) return;
		await api.clearFinished();
	}

	async function clearHistory() {
		if (!window.confirm(t("downloads.confirmClearHistory"))) return;
		await api.historyClear();
		await downloadsStore.loadHistory();
	}

	async function redownloadHistory(entry: HistoryEntry) {
		const s = settingsStore.value;
		try {
			await api.startDownload({
				url: entry.url,
				download_mode: entry.download_mode ?? "auto",
				quality: entry.quality ?? "best",
				format_id: null,
				playlist_items: null,
				output_dir: null
			});
			void s;
			await goto("/downloads");
			tab = "queue";
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	function filterLabel(id: Filter): string {
		switch (id) {
			case "all":
				return t("downloads.filterAll");
			case "active":
				return `${t("downloads.filterActive")} (${counts.active})`;
			case "queued":
				return `${t("downloads.filterQueued")} (${counts.queued})`;
			case "done":
				return t("downloads.filterDone");
			case "failed":
				return `${t("downloads.filterFailed")} (${counts.failed})`;
		}
	}

	function statusLabel(item: QueueItemInfo): string {
		switch (item.status) {
			case "queued":
				return t("downloads.statusQueued");
			case "active":
				return t("downloads.statusActive");
			case "paused":
				return t("downloads.statusPaused");
			case "done":
				return t("downloads.statusDone");
			case "error":
				return t("downloads.statusError");
		}
	}

	function platformInitial(item: { platform: string }): string {
		return (item.platform || "?").slice(0, 1).toUpperCase();
	}
</script>

<div class="page">
	<div class="page-inner">
		<header class="page-head">
			<h1 class="page-title">{t("downloads.title")}</h1>
			<div class="head-actions">
				<label class="limit-label" for="speed-limit">{t("downloads.speedLimit")}</label>
				<select
					id="speed-limit"
					class="input limit-select"
					value={String(settingsStore.value?.speed_limit_mbps ?? 0)}
					onchange={(e) =>
						settingsStore.update({
							speed_limit_mbps: Number((e.currentTarget as HTMLSelectElement).value)
						})}
				>
					<option value="0">{t("downloads.unlimited")}</option>
					{#each [1, 2, 5, 10] as mbps (mbps)}
						<option value={String(mbps)}>{mbps} MB/s</option>
					{/each}
				</select>
				{#if tab === "queue"}
					<button class="btn btn-sm btn-ghost" onclick={clearFinished}>{t("downloads.clearFinished")}</button>
				{:else}
					<button class="btn btn-sm btn-ghost" onclick={clearHistory}>{t("downloads.historyClear")}</button>
				{/if}
			</div>
		</header>

		<div class="segmented tabs" role="tablist">
			<button role="tab" aria-selected={tab === "queue"} onclick={() => (tab = "queue")}>
				{t("downloads.tabActive")}{counts.active + counts.queued > 0 ? ` (${counts.active + counts.queued})` : ""}
			</button>
			<button
				role="tab"
				aria-selected={tab === "history"}
				onclick={async () => {
					tab = "history";
					await downloadsStore.loadHistory();
				}}
			>
				{t("downloads.tabHistory")}
			</button>
		</div>

		{#if tab === "queue"}
			<div class="filters" role="group">
				{#each FILTERS as f (f.id)}
					<button class="pill pill-btn" class:active={filter === f.id} onclick={() => (filter = f.id)}>
						{filterLabel(f.id)}
					</button>
				{/each}
			</div>

			{#if filtered.length === 0}
				<div class="empty-state">
					<svg viewBox="0 0 256 256" aria-hidden="true"><g fill="none" stroke="var(--accent)" stroke-width="27" stroke-linecap="round" stroke-linejoin="round"><circle cx="81" cy="85" r="45"/><path d="M128 40 V152 Q128 189 91 189 Q62 189 50 163"/></g><path fill="var(--accent)" d="M88 194 Q184 190 228 92 L238 118 Q198 226 88 222 Z"/></svg>
					<h2 class="empty-title">{t("downloads.empty")}</h2>
					<p class="muted">{t("downloads.emptyHint")}</p>
					<a class="btn btn-accent" href="/">{t("downloads.emptyCta")}</a>
				</div>
			{:else}
				<ul class="item-list">
					{#each filtered as item (item.id)}
						<li class="card item">
							<div class="thumb" aria-hidden="true">
								{#if item.thumbnail_url}
									<img src={item.thumbnail_url} alt="" referrerpolicy="no-referrer" loading="lazy" />
								{:else}
									<span class="thumb-letter">{platformInitial(item)}</span>
								{/if}
							</div>
							<div class="item-body">
								<div class="item-top">
									<p class="item-title" title={item.title}>{item.title}</p>
									<span class="status status-{item.status}">{statusLabel(item)}</span>
								</div>
								<div class="item-meta muted">
									<span>{item.platform}</span>
									{#if item.total_bytes}<span>· {formatBytes(item.total_bytes)}</span>{/if}
									{#if item.status === "active" && item.speed_bps > 0}<span class="mono">· {formatSpeed(item.speed_bps)}</span>{/if}
									{#if item.status === "active" && item.eta_seconds != null}<span>· {t("downloads.eta", { time: formatEta(item.eta_seconds) })}</span>{/if}
								</div>
								{#if item.status === "active" || item.status === "paused"}
									<div class="progress" class:indeterminate={item.percent <= 0.1}><div style="width: {Math.min(100, Math.max(0, item.percent))}%"></div></div>
									{#if item.phase}<div class="phase muted">{item.phase}</div>{/if}
								{/if}
								{#if item.status === "error" && item.error_message}
									<div class="error-box">{item.error_message}</div>
								{/if}
							</div>
							<div class="item-actions">
								{#if item.status === "active"}
									<button class="btn-icon" title={t("downloads.pause")} aria-label={t("downloads.pause")} onclick={() => api.pauseDownload(item.id)}><svg viewBox="0 0 24 24"><path d="M9 5v14M15 5v14" /></svg></button>
									<button class="btn-icon" title={t("downloads.cancel")} aria-label={t("downloads.cancel")} onclick={() => api.cancelDownload(item.id)}><svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6 6 18" /></svg></button>
								{:else if item.status === "paused"}
									<button class="btn-icon" title={t("downloads.resume")} aria-label={t("downloads.resume")} onclick={() => api.resumeDownload(item.id)}><svg viewBox="0 0 24 24"><path d="M8 5l12 7-12 7z" /></svg></button>
									<button class="btn-icon" title={t("downloads.cancel")} aria-label={t("downloads.cancel")} onclick={() => api.cancelDownload(item.id)}><svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6 6 18" /></svg></button>
								{:else if item.status === "queued"}
									<button class="btn-icon" title={t("downloads.cancel")} aria-label={t("downloads.cancel")} onclick={() => api.cancelDownload(item.id)}><svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6 6 18" /></svg></button>
								{:else if item.status === "done"}
									<button class="btn-icon" title={t("downloads.reveal")} aria-label={t("downloads.reveal")} onclick={() => reveal(item.file_path)}><svg viewBox="0 0 24 24"><path d="M4 7a2 2 0 0 1 2-2h4l2 2.5h6a2 2 0 0 1 2 2V17a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2z" /></svg></button>
									<button class="btn-icon" title={t("downloads.remove")} aria-label={t("downloads.remove")} onclick={() => removeItem(item, false)}><svg viewBox="0 0 24 24"><path d="M5 7h14M10 11v6M14 11v6M8 7l1-2h6l1 2" /></svg></button>
									<button class="btn-icon danger" title={t("downloads.removeFile")} aria-label={t("downloads.removeFile")} onclick={() => removeItem(item, true)}><svg viewBox="0 0 24 24"><path d="M4 7h16M9 7V5h6v2M6 7l1.5 13a2 2 0 0 0 2 1.8h5a2 2 0 0 0 2-1.8L18 7" /></svg></button>
								{:else if item.status === "error"}
									{#if item.retryable}
										<button class="btn-icon" title={t("downloads.retry")} aria-label={t("downloads.retry")} onclick={() => api.retryDownload(item.id)}><svg viewBox="0 0 24 24"><path d="M3 12a9 9 0 1 0 2.6-6.4L3 8" /><path d="M3 3v5h5" /></svg></button>
									{/if}
									<button class="btn-icon" title={t("downloads.remove")} aria-label={t("downloads.remove")} onclick={() => removeItem(item, false)}><svg viewBox="0 0 24 24"><path d="M5 7h14M10 11v6M14 11v6M8 7l1-2h6l1 2" /></svg></button>
								{/if}
							</div>
						</li>
					{/each}
				</ul>
			{/if}
		{:else}
			{#if history.length === 0}
				<div class="empty-state">
					<p class="muted">{t("downloads.historyEmpty")}</p>
				</div>
			{:else}
				<ul class="item-list">
					{#each history as entry (entry.id)}
						<li class="card item">
							<div class="thumb" aria-hidden="true">
								{#if entry.thumbnail_url}
									<img src={entry.thumbnail_url} alt="" referrerpolicy="no-referrer" loading="lazy" />
								{:else}
									<span class="thumb-letter">{platformInitial(entry)}</span>
								{/if}
							</div>
							<div class="item-body">
								<div class="item-top">
									<p class="item-title" title={entry.title}>{entry.title}</p>
									<span class="status {entry.success ? 'status-done' : 'status-error'}">{entry.success ? t("downloads.statusDone") : t("downloads.statusError")}</span>
								</div>
								<div class="item-meta muted">
									<span>{entry.platform}</span>
									{#if entry.file_size_bytes}<span>· {formatBytes(entry.file_size_bytes)}</span>{/if}
									<span>· {formatRelativeTime(entry.completed_at)}</span>
								</div>
							</div>
							<div class="item-actions">
								<button class="btn-icon" title={t("downloads.redownload")} aria-label={t("downloads.redownload")} onclick={() => redownloadHistory(entry)}><svg viewBox="0 0 24 24"><path d="M3 12a9 9 0 1 0 2.6-6.4L3 8" /><path d="M3 3v5h5" /></svg></button>
								<button class="btn-icon" title={t("downloads.reveal")} aria-label={t("downloads.reveal")} onclick={() => reveal(entry.file_path)}><svg viewBox="0 0 24 24"><path d="M4 7a2 2 0 0 1 2-2h4l2 2.5h6a2 2 0 0 1 2 2V17a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2z" /></svg></button>
							</div>
						</li>
					{/each}
				</ul>
			{/if}
		{/if}
	</div>
</div>

<style>
	.page-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		margin-bottom: 14px;
		flex-wrap: wrap;
	}

	.page-title {
		font-family: var(--font-display);
		font-size: 1.9rem;
		font-weight: 700;
	}

	.head-actions {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.limit-label {
		font-size: 12px;
		color: var(--muted);
		font-weight: 600;
	}

	.limit-select {
		width: auto;
		padding: 7px 10px;
	}

	.tabs {
		margin-bottom: 12px;
	}

	.filters {
		display: flex;
		gap: 8px;
		margin-bottom: 14px;
		flex-wrap: wrap;
	}

	.filters .pill-btn.active {
		background: var(--accent-soft);
		color: var(--accent);
	}

	.empty-title {
		font-family: var(--font-display);
		font-size: 1.4rem;
		margin: 0 0 8px;
	}

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 8px;
	}

	.item-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.item {
		display: flex;
		gap: 14px;
		align-items: center;
		padding: 12px;
	}

	.thumb {
		width: 96px;
		height: 84px;
		border-radius: var(--r-md);
		overflow: hidden;
		flex: none;
		background: linear-gradient(135deg, var(--spruce), var(--accent-strong));
		display: grid;
		place-items: center;
	}

	.thumb img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.thumb-letter {
		font-family: var(--font-display);
		font-size: 2rem;
		font-weight: 800;
		color: var(--on-spruce);
		opacity: 0.9;
	}

	.item-body {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 5px;
	}

	.item-top {
		display: flex;
		align-items: baseline;
		gap: 10px;
		justify-content: space-between;
	}

	.item-title {
		margin: 0;
		font-weight: 600;
		font-size: 14px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.item-meta {
		font-size: 12px;
		display: flex;
		gap: 4px;
		flex-wrap: wrap;
	}

	.phase {
		font-size: 11px;
		text-transform: lowercase;
	}

	.status {
		font-size: 10px;
		font-weight: 800;
		text-transform: uppercase;
		letter-spacing: 0.07em;
		flex: none;
		padding: 3px 8px;
		border-radius: var(--r-pill);
	}

	.status-active {
		color: var(--accent);
		background: var(--accent-soft);
	}

	.status-queued {
		color: var(--muted);
		background: var(--card-2);
	}

	.status-paused {
		color: var(--warn);
		background: var(--warn-soft);
	}

	.status-done {
		color: var(--ok);
		background: var(--ok-soft);
	}

	.status-error {
		color: var(--danger);
		background: var(--danger-soft);
	}

	.error-box {
		font-size: 12px;
		color: var(--danger);
		background: var(--danger-soft);
		border-radius: var(--r-sm);
		padding: 6px 8px;
		word-break: break-word;
	}

	.item-actions {
		display: flex;
		flex-direction: column;
		gap: 6px;
		flex: none;
	}

	.btn-icon.danger {
		color: var(--danger);
	}

	.btn-icon.danger:hover {
		background: var(--danger-soft);
	}

	@media (max-width: 640px) {
		.thumb {
			width: 72px;
			height: 72px;
		}

		.item-actions {
			flex-direction: row;
		}
	}
</style>
