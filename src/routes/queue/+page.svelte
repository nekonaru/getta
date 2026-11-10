<script lang="ts">
	import { api, type HistoryEntry, type QueueItemInfo } from "#lib/api";
	import { humanize } from "#lib/errors";
	import { downloadsStore } from "#lib/stores/downloads.svelte";
	import { settingsStore } from "#lib/stores/settings.svelte";
	import { toastStore } from "#lib/stores/toast.svelte";
	import { t } from "#lib/i18n.svelte";
	import { formatBytes, formatEta, formatRelativeTime, formatSpeed } from "#lib/format";
	import Sparkline from "#lib/components/Sparkline.svelte";

	type Tab = "queue" | "history";
	let tab = $state<Tab>("queue");

	const transit = $derived(
		[...downloadsStore.items]
			.filter((i) => i.status === "active" || i.status === "paused")
			.sort((a, b) => {
				const rank = (s: QueueItemInfo["status"]) => (s === "active" ? 0 : 1);
				const diff = rank(a.status) - rank(b.status);
				return diff !== 0 ? diff : b.id - a.id;
			})
	);
	const queued = $derived(
		[...downloadsStore.items].filter((i) => i.status === "queued").sort((a, b) => a.id - b.id)
	);
	const delivered = $derived(
		[...downloadsStore.items].filter((i) => i.status === "done").sort((a, b) => b.id - a.id)
	);
	const failed = $derived(
		[...downloadsStore.items].filter((i) => i.status === "error").sort((a, b) => b.id - a.id)
	);

	const aggregate = $derived.by(() => {
		const activeItems = transit.filter((i) => i.status === "active");
		if (activeItems.length === 0) return null;
		const speed = activeItems.reduce((sum, i) => sum + i.speed_bps, 0);
		const etas = activeItems.map((i) => i.eta_seconds).filter((v): v is number => v != null);
		const percentages = activeItems.map((i) => i.percent);
		// Prefer byte-accurate math when every active item knows its size.
		const bytesKnown = activeItems.every((i) => (i.total_bytes ?? 0) > 0 && i.downloaded_bytes != null);
		let percent: number;
		if (bytesKnown) {
			const total = activeItems.reduce((sum, i) => sum + (i.total_bytes ?? 0), 0);
			const done = activeItems.reduce((sum, i) => sum + i.downloaded_bytes, 0);
			percent = total > 0 ? (done / total) * 100 : 0;
		} else {
			percent = percentages.reduce((sum, p) => sum + p, 0) / percentages.length;
		}
		const eta = etas.length ? Math.max(...etas) : null;
		return { speed, eta, percent: Math.min(100, Math.max(0, percent)), count: activeItems.length };
	});

	const history = $derived(downloadsStore.history);

	const sections = $derived([
		{ id: "transit", label: () => t("queue.sectionTransit"), list: transit },
		{ id: "queued", label: () => t("queue.sectionQueued"), list: queued },
		{ id: "delivered", label: () => t("queue.sectionDelivered"), list: delivered },
		{ id: "failed", label: () => t("queue.sectionFailed"), list: failed }
	]);

	async function reveal(path: string | null) {
		if (!path) return;
		try {
			await api.revealInFolder(path);
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function removeItem(item: QueueItemInfo, deleteFile: boolean) {
		try {
			await api.removeDownload(item.id, deleteFile);
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function confirmRemoveFile(item: QueueItemInfo) {
		if (!window.confirm(t("queue.confirmRemoveFile", { title: item.title }))) return;
		await removeItem(item, true);
	}

	async function clearFinished() {
		if (!window.confirm(t("queue.clearFinishedConfirm"))) return;
		try {
			await api.clearFinished();
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function exportHistory(format: "csv" | "json") {
		try {
			const { save } = await import("@tauri-apps/plugin-dialog");
			const path = await save({
				defaultPath: format === "csv" ? "getta-history.csv" : "getta-history.json",
				filters: [
					format === "csv"
						? { name: "CSV", extensions: ["csv"] }
						: { name: "JSON", extensions: ["json"] }
				]
			});
			if (!path) return;
			if (format === "csv") {
				await api.exportHistoryCsv(path);
			} else {
				await api.exportHistoryJson(path);
			}
			toastStore.push(t("queue.exportCsvDone"));
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function clearHistory() {
		if (!window.confirm(t("queue.clearHistoryConfirm"))) return;
		try {
			await api.historyClear();
			await downloadsStore.loadHistory();
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function redownload(entry: HistoryEntry) {
		try {
			await api.startDownload({
				url: entry.url,
				download_mode: entry.download_mode ?? "auto",
				quality: entry.quality ?? "best",
				format_id: null,
				playlist_items: null,
				output_dir: null
			});
			tab = "queue";
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function switchTab(next: Tab) {
		tab = next;
		if (next === "history") await downloadsStore.loadHistory();
	}

	function statusLabel(item: QueueItemInfo): string {
		switch (item.status) {
			case "queued":
				return t("queue.statusQueued");
			case "active":
				return t("queue.statusActive");
			case "paused":
				return t("queue.statusPaused");
			case "done":
				return t("queue.statusDone");
			case "error":
				return t("queue.statusError");
		}
	}

	function platformInitial(platform: string): string {
		return (platform || "?").slice(0, 1).toUpperCase();
	}
</script>

<div class="page">
	<div class="page-inner">
		<header class="page-head">
			<div>
				<p class="kicker">{t("queue.kicker")}</p>
				<h1 class="page-title">{t("queue.title")}</h1>
			</div>
			<div class="head-actions">
				<label class="speed-label" for="speed-limit">{t("queue.speedLimit")}</label>
				<select
					id="speed-limit"
					class="input limit-select"
					value={String(settingsStore.value?.speed_limit_mbps ?? 0)}
					onchange={(e) =>
						settingsStore.update({
							speed_limit_mbps: Number((e.currentTarget as HTMLSelectElement).value)
						})}
				>
					<option value="0">{t("queue.unlimited")}</option>
					{#each [1, 2, 5, 10] as mbps (mbps)}
						<option value={String(mbps)}>{mbps} MB/s</option>
					{/each}
				</select>
				{#if tab === "queue"}
					<button class="btn btn-sm btn-ghost" onclick={clearFinished}>{t("queue.clearFinished")}</button>
				{:else}
					<button class="btn btn-sm btn-ghost" onclick={() => exportHistory("csv")}>{t("queue.exportCsv")}</button>
					<button class="btn btn-sm btn-ghost" onclick={() => exportHistory("json")}>{t("queue.exportJson")}</button>
					<button class="btn btn-sm btn-ghost" onclick={clearHistory}>{t("queue.clearHistory")}</button>
				{/if}
			</div>
		</header>

		<div class="segmented tabs" role="tablist">
			<button role="tab" aria-selected={tab === "queue"} onclick={() => switchTab("queue")}>
				{t("queue.tabQueue")}
			</button>
			<button role="tab" aria-selected={tab === "history"} onclick={() => switchTab("history")}>
				{t("queue.tabHistory")}
			</button>
		</div>

		{#if tab === "queue"}
			{#if aggregate}
				{@const agg = aggregate}
				<div class="aggregate-wrap">
					<div class="aggregate mono" aria-live="polite">
						{t("queue.aggLine", { active: agg.count, speed: formatSpeed(agg.speed), eta: agg.eta != null ? formatEta(agg.eta) : "…" })}
					</div>
					<div class="progress aggregate-bar" role="progressbar" aria-valuenow={Math.round(agg.percent)} aria-valuemin="0" aria-valuemax="100">
						<div class="progress-fill progress-sheen" style="width: {agg.percent}%"></div>
					</div>
				</div>
			{/if}

			{#if downloadsStore.items.length === 0}
				<div class="empty-state">
					<svg viewBox="0 0 256 256" width="88" height="88" aria-hidden="true"><g fill="none" stroke="var(--accent)" stroke-width="24" stroke-linecap="round" stroke-linejoin="round"><circle cx="81" cy="85" r="45"/><path d="M128 40 V152 Q128 189 91 189 Q62 189 50 163"/></g><path fill="var(--accent)" d="M88 194 Q184 190 228 92 L238 118 Q198 226 88 222 Z"/></svg>
					<h2 class="empty-title">{t("queue.empty")}</h2>
					<p class="muted">{t("queue.emptyHint")}</p>
					<a class="btn btn-accent" href="/">{t("queue.emptyCta")}</a>
				</div>
			{:else}
				{#each sections as section (section.id)}
					{#if section.list.length > 0}
						<section class="manifest-section">
							<div class="section-head">
								<h2 class="section-label">{section.label()}</h2>
								<span class="section-count mono">{section.list.length}</span>
							</div>
							<div class="manifest card">
								{#each section.list as item (item.id)}
									<div class="manifest-entry" class:is-active={item.status === "active"}>
										<div class="row-main">
											<div class="thumb" aria-hidden="true">
												{#if item.thumbnail_url}
													<img src={item.thumbnail_url} alt="" referrerpolicy="no-referrer" loading="lazy" />
												{:else}
													<span>{platformInitial(item.platform)}</span>
												{/if}
											</div>
											<div class="row-body">
												<div class="row-top">
													<p class="row-title" title={item.title}>{item.title}</p>
													<span class="state state-{item.status}">{statusLabel(item)}</span>
												</div>
												<div class="row-meta muted">
													<span class="mono">{item.platform}</span>
													{#if item.total_bytes}<span>· {formatBytes(item.downloaded_bytes)} / {formatBytes(item.total_bytes)}</span>{/if}
													{#if item.status === "active" && item.speed_bps > 0}<span class="mono">· {formatSpeed(item.speed_bps)}</span>{/if}
													{#if item.status === "active" && item.eta_seconds != null}<span>· {t("queue.eta", { time: formatEta(item.eta_seconds) })}</span>{/if}
													{#if item.status === "paused"}<span>· {formatBytes(item.downloaded_bytes)}</span>{/if}
									{#if (item.status === "active" || item.status === "queued") && item.phase && item.phase !== "downloading"}<span class="phase-chip mono">{item.phase}</span>{/if}
												</div>
												{#if item.error_message}
													<div class="error-box">{item.error_message}</div>
												{/if}
											</div>
											<div class="row-side">
												{#if item.status === "active"}
													<span class="percent-big mono">{Math.floor(item.percent)}<span class="percent-sign">%</span></span>
													<Sparkline samples={downloadsStore.speedHistory.get(item.id) ?? []} />
												{:else if item.status === "paused"}
													<span class="percent-big mono">{Math.floor(item.percent)}<span class="percent-sign">%</span></span>
												{/if}
											</div>
											<div class="row-actions">
												{#if item.status === "active"}
													<button class="btn-icon" title={t("queue.pause")} aria-label={t("queue.pause")} onclick={() => api.pauseDownload(item.id)}><svg viewBox="0 0 24 24"><path d="M9 5v14M15 5v14" /></svg></button>
													<button class="btn-icon" title={t("queue.cancel")} aria-label={t("queue.cancel")} onclick={() => api.cancelDownload(item.id)}><svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6 6 18" /></svg></button>
												{:else if item.status === "paused"}
													<button class="btn-icon" title={t("queue.resume")} aria-label={t("queue.resume")} onclick={() => api.resumeDownload(item.id)}><svg viewBox="0 0 24 24"><path d="M8 5l12 7-12 7z" /></svg></button>
													<button class="btn-icon" title={t("queue.cancel")} aria-label={t("queue.cancel")} onclick={() => api.cancelDownload(item.id)}><svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6 6 18" /></svg></button>
												{:else if item.status === "queued"}
													<button class="btn-icon" title={t("queue.moveUp")} aria-label={t("queue.moveUp")} onclick={() => api.moveItem(item.id, -1)}><svg viewBox="0 0 24 24"><path d="M12 19V5" /><path d="m6 11 6-6 6 6" /></svg></button>
													<button class="btn-icon" title={t("queue.moveDown")} aria-label={t("queue.moveDown")} onclick={() => api.moveItem(item.id, 1)}><svg viewBox="0 0 24 24"><path d="M12 5v14" /><path d="m18 13-6 6-6-6" /></svg></button>
													<button class="btn-icon" title={t("queue.cancel")} aria-label={t("queue.cancel")} onclick={() => api.cancelDownload(item.id)}><svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6 6 18" /></svg></button>
												{:else if item.status === "done"}
													<button class="btn-icon" title={t("queue.reveal")} aria-label={t("queue.reveal")} onclick={() => reveal(item.file_path)}><svg viewBox="0 0 24 24"><path d="M4 8V6a2 2 0 0 1 2-2h4l2 2.5h6a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2z" /></svg></button>
													<button class="btn-icon" title={t("queue.remove")} aria-label={t("queue.remove")} onclick={() => removeItem(item, false)}><svg viewBox="0 0 24 24"><path d="M5 7h14M10 11v6M14 11v6M8 7l1-2h6l1 2" /></svg></button>
													<button class="btn-icon danger" title={t("queue.removeFile")} aria-label={t("queue.removeFile")} onclick={() => confirmRemoveFile(item)}><svg viewBox="0 0 24 24"><path d="M4 7h16M9 7V5h6v2M6 7l1.5 13a2 2 0 0 0 2 1.8h5a2 2 0 0 0 2-1.8L18 7" /></svg></button>
												{:else if item.status === "error"}
													{#if item.retryable}
														<button class="btn-icon" title={t("queue.retry")} aria-label={t("queue.retry")} onclick={() => api.retryDownload(item.id)}><svg viewBox="0 0 24 24"><path d="M3 12a9 9 0 1 0 2.6-6.4L3 8" /><path d="M3 3v5h5" /></svg></button>
													{/if}
													<button class="btn-icon" title={t("queue.remove")} aria-label={t("queue.remove")} onclick={() => removeItem(item, false)}><svg viewBox="0 0 24 24"><path d="M5 7h14M10 11v6M14 11v6M8 7l1-2h6l1 2" /></svg></button>
												{/if}
											</div>
										</div>
										{#if item.status === "active" || item.status === "paused"}
											<div class="progress" class:indeterminate={item.percent <= 0.1}><div class="progress-fill" class:progress-sheen={item.status === "active" && item.percent > 0.1} style="width: {Math.min(100, Math.max(0, item.percent))}%"></div></div>
										{/if}
									</div>
								{/each}
							</div>
						</section>
					{/if}
				{/each}
			{/if}
		{:else}
			{#if history.length === 0}
				<div class="empty-state">
					<p class="muted">{t("queue.historyEmpty")}</p>
				</div>
			{:else}
				<div class="manifest card">
					{#each history as entry (entry.id)}
						<div class="manifest-entry">
							<div class="row-main">
								<div class="thumb" aria-hidden="true">
									{#if entry.thumbnail_url}
										<img src={entry.thumbnail_url} alt="" referrerpolicy="no-referrer" loading="lazy" />
									{:else}
										<span>{platformInitial(entry.platform)}</span>
									{/if}
								</div>
								<div class="row-body">
									<div class="row-top">
										<p class="row-title" title={entry.title}>{entry.title}</p>
										<span class="state {entry.success ? 'state-done' : 'state-error'}">{entry.success ? t("queue.statusDone") : t("queue.statusError")}</span>
									</div>
									<div class="row-meta muted">
										<span class="mono">{entry.platform}</span>
										{#if entry.file_size_bytes}<span>· {formatBytes(entry.file_size_bytes)}</span>{/if}
										<span>· {formatRelativeTime(entry.completed_at)}</span>
									</div>
								</div>
								<div class="row-actions">
									<button class="btn-icon" title={t("queue.redownload")} aria-label={t("queue.redownload")} onclick={() => redownload(entry)}><svg viewBox="0 0 24 24"><path d="M3 12a9 9 0 1 0 2.6-6.4L3 8" /><path d="M3 3v5h5" /></svg></button>
									<button class="btn-icon" title={t("queue.reveal")} aria-label={t("queue.reveal")} onclick={() => reveal(entry.file_path)}><svg viewBox="0 0 24 24"><path d="M4 8V6a2 2 0 0 1 2-2h4l2 2.5h6a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2z" /></svg></button>
								</div>
							</div>
						</div>
					{/each}
				</div>
			{/if}
		{/if}
	</div>
</div>

<style>
	.head-actions {
		display: flex;
		align-items: center;
		gap: 10px;
		flex-wrap: wrap;
	}

	.speed-label {
		font-family: var(--font-mono);
		font-size: 9.5px;
		letter-spacing: 0.18em;
		text-transform: uppercase;
		color: var(--muted);
	}

	.limit-select {
		width: auto;
		padding: 6px 10px;
		font-size: 12px;
	}

	.tabs {
		margin-bottom: 12px;
	}

	.aggregate {
		font-size: 12px;
		font-weight: 600;
		color: var(--accent);
		border: 1px solid var(--accent);
		background: var(--accent-soft);
		border-radius: var(--r-sm) var(--r-sm) 0 0;
		padding: 8px 12px 4px;
		margin-bottom: 0;
		letter-spacing: 0.02em;
	}

	.aggregate-wrap {
		margin-bottom: 16px;
	}

	.aggregate-bar {
		height: 5px;
		border-radius: 0 0 var(--r-sm) var(--r-sm);
	}

	.manifest-section + .manifest-section {
		margin-top: 22px;
	}

	.section-head {
		display: flex;
		align-items: baseline;
		gap: 10px;
		border-bottom: 2px solid var(--border);
		padding-bottom: 6px;
		margin-bottom: 0;
	}

	.section-label {
		font-family: var(--font-mono);
		font-size: 10.5px;
		letter-spacing: 0.22em;
		text-transform: uppercase;
		color: var(--text);
	}

	.section-count {
		font-size: 11px;
		color: var(--muted);
	}

	.manifest {
		padding: 0;
		overflow: hidden;
	}

	.manifest-entry {
		padding: 12px 14px 0;
		border-bottom: 1px solid var(--border-soft);
	}

	.manifest-entry:last-child {
		border-bottom: none;
	}

	.manifest-entry.is-active {
		background: var(--accent-faint);
		box-shadow: inset 2px 0 0 var(--accent);
	}

	.row-main {
		display: grid;
		grid-template-columns: 84px minmax(0, 1fr) auto auto;
		gap: 12px;
		align-items: center;
	}

	.thumb {
		width: 84px;
		height: 62px;
		background: var(--card-2);
		border: 1px solid var(--border-soft);
		display: grid;
		place-items: center;
		font-family: var(--font-display);
		font-weight: 800;
		color: var(--muted);
		overflow: hidden;
	}

	.thumb img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.row-body {
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 3px;
	}

	.row-top {
		display: flex;
		align-items: baseline;
		gap: 10px;
		justify-content: space-between;
	}

	.row-title {
		margin: 0;
		font-weight: 620;
		font-size: 13px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.row-meta {
		font-size: 11px;
		display: flex;
		gap: 4px;
		flex-wrap: wrap;
		align-items: center;
	}

	.phase-chip {
		font-size: 9px;
		color: var(--accent);
		background: var(--accent-soft);
		border-radius: 3px;
		padding: 1px 5px;
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}

	.state {
		font-family: var(--font-mono);
		font-size: 8.5px;
		font-weight: 700;
		letter-spacing: 0.14em;
		text-transform: uppercase;
		flex: none;
	}

	.state-active {
		color: var(--accent);
	}

	.state-queued {
		color: var(--muted);
	}

	.state-paused {
		color: var(--warn);
	}

	.state-done {
		color: var(--good);
	}

	.state-error {
		color: var(--bad);
	}

	.error-box {
		font-size: 11.5px;
		color: var(--bad);
		margin-top: 2px;
	}

	.row-side {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 4px;
		min-width: 120px;
	}

	.percent-big {
		font-size: 24px;
		font-weight: 500;
		line-height: 1;
	}

	.percent-sign {
		font-size: 12px;
		color: var(--muted);
	}

	.row-actions {
		display: flex;
		gap: 2px;
	}

	.manifest-entry .progress {
		margin: 10px -14px 0;
	}

	@media (max-width: 760px) {
		.row-main {
			grid-template-columns: 64px minmax(0, 1fr);
			grid-template-areas:
				"thumb body"
				"side actions";
		}

		.thumb {
			grid-area: thumb;
			width: 64px;
			height: 52px;
		}

		.row-body {
			grid-area: body;
		}

		.row-side {
			grid-area: side;
			align-items: flex-start;
			flex-direction: row;
			gap: 12px;
		}

		.row-actions {
			grid-area: actions;
			justify-self: end;
		}
	}
</style>

