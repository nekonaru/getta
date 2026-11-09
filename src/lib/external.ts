import { openUrl } from "@tauri-apps/plugin-opener";

/** Open an external URL in the system browser. Falls back to window.open in a plain browser. */
export async function openExternal(url: string): Promise<void> {
	try {
		await openUrl(url);
		return;
	} catch (error) {
		console.debug("opener plugin unavailable, using window.open", error);
	}
	try {
		window.open(url, "_blank", "noopener,noreferrer");
	} catch {
		/* nothing else we can do */
	}
}
