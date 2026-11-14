import { api } from "#lib/api";
import { settingsStore } from "#lib/stores/settings.svelte";
import { toastStore } from "#lib/stores/toast.svelte";
import { t } from "#lib/i18n.svelte";
import { open } from "@tauri-apps/plugin-dialog";
import { extractLink } from "#lib/url";

export interface EnqueueResult {
	queued: boolean;
	url: string;
}

/** Why an enqueue attempt did not happen. */
export type EnqueueFailure = "empty-clipboard" | "no-link";

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

export async function enqueueText(
	text: string,
	forceFolder?: boolean
): Promise<{ queued: boolean; reason: EnqueueFailure | null }> {
	const match = extractLink(text);
	if (!match) {
		return { queued: false, reason: text.trim() ? "no-link" : "empty-clipboard" };
	}
	const result = await enqueueUrl(match.url, forceFolder);
	return { queued: result.queued, reason: null };
}

export async function enqueueFromClipboardText(text: string, forceFolder?: boolean): Promise<boolean> {
	const result = await enqueueText(text, forceFolder);
	if (result.queued) {
		toastStore.push(t("common.queued"), "info");
		return true;
	}
	if (result.reason === "empty-clipboard") {
		toastStore.push(t("enqueue.emptyClipboard"), "error");
		return false;
	}
	toastStore.push(t("home.invalidUrl"), "error");
	return false;
}
