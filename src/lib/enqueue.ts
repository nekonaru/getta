import { api } from "#lib/api";
import { settingsStore } from "#lib/stores/settings.svelte";
import { toastStore } from "#lib/stores/toast.svelte";
import { t } from "#lib/i18n.svelte";
import { open } from "@tauri-apps/plugin-dialog";

export interface EnqueueResult {
	queued: boolean;
	url: string;
}

function firstUrl(text: string): string | null {
	const found = text
		.trim()
		.split(/\s+/)
		.find((token) => /^(https?:\/\/|magnet:|p2p:)/.test(token));
	return found ?? null;
}

export async function enqueueUrl(url: string, forceFolder?: boolean): Promise<EnqueueResult> {
	const settings = settingsStore.value;
	let outputDir: string | null = null;
	if (forceFolder || settings?.always_ask_path) {
		try {
			const picked = await open({ directory: true, multiple: false });
			if (typeof picked !== "string" || !picked) return { queued: false, url };
			outputDir = picked;
		} catch {
			return { queued: false, url };
		}
	}
	try {
		await api.startDownload({
			url,
			download_mode: "auto",
			quality: settings?.video_quality ?? "best",
			format_id: null,
			playlist_items: null,
			output_dir: outputDir
		});
		return { queued: true, url };
	} catch (e) {
		toastStore.push(e instanceof Error ? e.message : String(e), "error");
		return { queued: false, url };
	}
}

export async function enqueueFromClipboardText(text: string, forceFolder?: boolean): Promise<boolean> {
	const url = firstUrl(text ?? "");
	if (!url) {
		toastStore.push(t("home.invalidUrl"), "error");
		return false;
	}
	const result = await enqueueUrl(url, forceFolder);
	if (result.queued) toastStore.push(t("common.queued"), "info");
	return result.queued;
}
