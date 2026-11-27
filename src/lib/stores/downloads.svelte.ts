import {
	api,
	onQueueChanged,
	type HistoryEntry,
	type QueueItemInfo,
	type QueueStatusName
} from "#lib/api";
import { settingsStore } from "#lib/stores/settings.svelte";
import { toastStore } from "#lib/stores/toast.svelte";
import { t } from "#lib/i18n.svelte";
import {
	isPermissionGranted,
	requestPermission,
	sendNotification
} from "@tauri-apps/plugin-notification";
import { humanize } from "#lib/errors";

const STATS_KEY = "getta.stats";
const HISTORY_LEN = 48;

export function readStats(): { downloads: number; bytes: number } {
	try {
		const raw = localStorage.getItem(STATS_KEY);
		if (raw) return JSON.parse(raw);
	} catch {
		/* ignore */
	}
	return { downloads: 0, bytes: 0 };
}

function writeStats(stats: { downloads: number; bytes: number }): void {
	try {
		localStorage.setItem(STATS_KEY, JSON.stringify(stats));
	} catch {
		/* ignore */
	}
}

class DownloadsStore {
	items = $state<QueueItemInfo[]>([]);
	loaded = $state(false);
	/** Ring buffer of recent speed samples (bytes/s), per item id. */
	speedHistory = $state<Map<number, number[]>>(new Map());
	history = $state<HistoryEntry[]>([]);
	private started = false;
	private unlisten: (() => void) | null = null;
	private historyLoaded = false;
	private lastEventAt = 0;

	get activeCount(): number {
		return this.items.filter((i) => i.status === "active").length;
	}

	get queuedCount(): number {
		return this.items.filter((i) => i.status === "queued").length;
	}

	get doneCount(): number {
		return this.items.filter((i) => i.status === "done").length;
	}

	get failedCount(): number {
		return this.items.filter((i) => i.status === "error").length;
	}

	get totalSpeed(): number {
		return this.items.reduce((sum, i) => sum + (i.status === "active" ? i.speed_bps : 0), 0);
	}

	async start(): Promise<void> {
		if (this.started) return;
		this.started = true;
		try {
			this.items = await api.queueSnapshot();
			this.loaded = true;
		} catch {
			/* backend not ready; listener below will populate */
		}
		try {
			this.unlisten = await onQueueChanged((items) => {
				this.lastEventAt = Date.now();
				// On a cold start the snapshot may have failed. Seed the first
				// event instead of treating every existing item as a new completion.
				const seeded = !this.loaded;
				const source = seeded ? items : this.items;
				const previous = new Map(source.map((i) => [i.id, i.status]));
				this.items = items;
				this.loaded = true;
				this.recordSpeed(items);
				if (!seeded) this.trackTransitions(previous, items);
			});
		} catch {
			// Registration failed. Allow a later retry instead of latching
			// forever with no live updates.
			this.started = false;
		}
	}

	private recordSpeed(items: QueueItemInfo[]): void {
		const next = new Map(this.speedHistory);
		const active = new Set<number>();
		for (const item of items) {
			if (item.status !== "active") continue;
			active.add(item.id);
			const history = next.get(item.id);
			if (history) {
				history.push(item.speed_bps);
				if (history.length > HISTORY_LEN) history.splice(0, history.length - HISTORY_LEN);
			} else {
				next.set(item.id, [item.speed_bps]);
			}
		}
		for (const id of next.keys()) {
			if (!active.has(id)) next.delete(id);
		}
		this.speedHistory = next;
	}

	/// Re-read the queue snapshot. Needed after backend actions that do not
	/// emit queue-changed (batch updates, imports, CLI pause/resume).
	async refresh(): Promise<void> {
		const requestedAt = Date.now();
		try {
			const items = await api.queueSnapshot();
			// A completion event may have arrived while the snapshot was in
			// flight. Never overwrite newer state with an older snapshot.
			if (this.lastEventAt > requestedAt) return;
			this.items = items;
			this.loaded = true;
		} catch {
			/* backend not ready */
		}
	}

	private trackTransitions(previous: Map<number, QueueStatusName>, items: QueueItemInfo[]): void {
		for (const item of items) {
			const prev = previous.get(item.id);
			if (prev === item.status) continue;

			if (item.status === "done") {
				const stats = readStats();
				stats.downloads += 1;
				stats.bytes += item.total_bytes ?? item.downloaded_bytes ?? 0;
				writeStats(stats);

				if (settingsStore.value?.notifications) {
					// Skip when the window is focused; the toast already confirms it.
					const focused = typeof document !== "undefined" && document.hasFocus?.();
					if (!focused) {
						void (async () => {
							try {
								let granted = await isPermissionGranted();
								if (!granted) granted = (await requestPermission()) === "granted";
								if (granted) {
									sendNotification({ title: t("queue.statusDone"), body: item.title });
								}
							} catch {
								/* notifications unavailable */
							}
						})();
					}
				}
				toastStore.push(`${t("queue.statusDone")}: ${item.title}`);
			}

			if (item.status === "error") {
				toastStore.push(`${t("queue.statusError")}: ${humanize(item.error_message)}`, "error");
			}
		}
	}

	async loadHistory(force = false): Promise<void> {
		if (this.historyLoaded && !force) return;
		try {
			this.history = await api.historyList();
			this.historyLoaded = true;
		} catch {
			/* keep the flag false so a later tab visit retries */
		}
	}
}

export const downloadsStore = new DownloadsStore();
