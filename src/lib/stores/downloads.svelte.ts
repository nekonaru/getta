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

const STATS_KEY = "getta.stats";

function readStats(): { downloads: number; bytes: number } {
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
	private started = false;

	get activeCount(): number {
		return this.items.filter((i) => i.status === "active" || i.status === "queued").length;
	}

	get doneCount(): number {
		return this.items.filter((i) => i.status === "done").length;
	}

	get failedCount(): number {
		return this.items.filter((i) => i.status === "error").length;
	}

	sorted(statusOrder: QueueStatusName[]): QueueItemInfo[] {
		const order = new Map(statusOrder.map((s, i) => [s, i]));
		return [...this.items].sort((a, b) => {
			const oa = order.get(a.status) ?? 99;
			const ob = order.get(b.status) ?? 99;
			if (oa !== ob) return oa - ob;
			return b.id - a.id;
		});
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
		await onQueueChanged((items) => {
			const previous = new Map(this.items.map((i) => [i.id, i.status]));
			this.items = items;
			this.loaded = true;
			this.trackTransitions(previous, items);
		});
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
					void (async () => {
						try {
							let granted = await isPermissionGranted();
							if (!granted) granted = (await requestPermission()) === "granted";
							if (granted) {
								sendNotification({ title: t("downloads.statusDone"), body: item.title });
							}
						} catch {
							/* notifications unavailable */
						}
					})();
				}
				toastStore.push(`${t("downloads.statusDone")}: ${item.title}`);
			}

			if (item.status === "error") {
				toastStore.push(
					`${t("downloads.statusError")}: ${item.error_message ?? item.title}`,
					"error"
				);
			}
		}
	}

	history: HistoryEntry[] = [];

	async loadHistory(): Promise<void> {
		this.history = await api.historyList();
	}
}

export const downloadsStore = new DownloadsStore();
