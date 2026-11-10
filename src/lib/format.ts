export function formatBytes(bytes: number | null | undefined): string {
	if (bytes == null || Number.isNaN(bytes)) return "-";
	if (bytes < 1024) return `${bytes} B`;
	const units = ["KB", "MB", "GB", "TB"];
	let value = bytes;
	let unit = "B";
	for (const next of units) {
		if (value < 1024) break;
		value /= 1024;
		unit = next;
	}
	return `${value >= 100 ? value.toFixed(0) : value.toFixed(1)} ${unit}`;
}

export function formatSpeed(bytesPerSecond: number): string {
	if (!bytesPerSecond) return "-";
	return `${formatBytes(bytesPerSecond)}/s`;
}

export function formatEta(seconds: number | null | undefined): string {
	if (seconds == null || !Number.isFinite(seconds)) return "-";
	const s = Math.max(0, Math.round(seconds));
	if (s < 60) return `${s}s`;
	const m = Math.floor(s / 60);
	if (m < 60) return `${m}m ${s % 60}s`;
	const h = Math.floor(m / 60);
	return `${h}h ${m % 60}m`;
}

export function formatDuration(seconds: number | null | undefined): string {
	if (seconds == null || !Number.isFinite(seconds)) return "-";
	const s = Math.max(0, Math.floor(seconds));
	const h = Math.floor(s / 3600);
	const m = Math.floor((s % 3600) / 60);
	const sec = s % 60;
	if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(sec).padStart(2, "0")}`;
	return `${m}:${String(sec).padStart(2, "0")}`;
}

export function formatRelativeTime(unixSeconds: number): string {
	const diff = Math.max(0, Math.floor(Date.now() / 1000 - unixSeconds));
	if (diff < 60) return "just now";
	if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
	if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
	if (diff < 86400 * 7) return `${Math.floor(diff / 86400)}d ago`;
	return new Date(unixSeconds * 1000).toLocaleDateString();
}
