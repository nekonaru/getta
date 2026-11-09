import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type DownloadMode = "auto" | "audio" | "mute";
export type QueueStatusName = "queued" | "active" | "paused" | "done" | "error";

export interface VideoQuality {
	label: string;
	width: number | null;
	height: number | null;
	format: string | null;
}

export interface PlaylistEntry {
	index: number;
	title: string;
	url: string;
}

export interface Analysis {
	platform: string;
	supported: boolean;
	title: string | null;
	author: string | null;
	thumbnail_url: string | null;
	duration_seconds: number | null;
	media_kind: string | null;
	is_playlist: boolean;
	entry_count: number | null;
	qualities: VideoQuality[];
	playlist_entries: PlaylistEntry[];
}

export interface QueueItemInfo {
	id: number;
	url: string;
	title: string;
	platform: string;
	status: QueueStatusName;
	error_message: string | null;
	retryable: boolean;
	percent: number;
	speed_bps: number;
	downloaded_bytes: number;
	total_bytes: number | null;
	eta_seconds: number | null;
	phase: string | null;
	file_path: string | null;
	file_count: number | null;
	thumbnail_url: string | null;
	author: string | null;
	duration_seconds: number | null;
	quality: string | null;
	download_mode: DownloadMode | null;
	format_id: string | null;
}

export interface HistoryEntry {
	id: string;
	url: string;
	platform: string;
	title: string;
	file_path: string | null;
	file_size_bytes: number | null;
	success: boolean;
	error: string | null;
	completed_at: number;
	thumbnail_url: string | null;
	quality: string | null;
	download_mode: DownloadMode | null;
}

export interface AppSettings {
	output_dir: string;
	always_ask_path: boolean;
	organize_by_platform: boolean;
	skip_existing: boolean;
	filename_template: string;
	video_quality: string;
	max_concurrent: number;
	concurrent_fragments: number;
	max_retries: number;
	speed_limit_mbps: number;
	embed_metadata: boolean;
	embed_thumbnail: boolean;
	write_subtitles: boolean;
	subtitle_lang: string;
	sponsorblock: boolean;
	theme: "system" | "light" | "dark";
	language: "en" | "id";
	reduce_motion: boolean;
	autostart: boolean;
	onboarding_completed: boolean;
	cookies_from_browser: string;
	cookie_file: string;
	proxy: string;
	notifications: boolean;
	prevent_sleep: boolean;
	ytdlp_path: string | null;
	ffmpeg_path: string | null;
}

export interface DependencyInfo {
	name: string;
	found: boolean;
	version: string | null;
	source: "managed" | "system" | "custom" | null;
	path: string | null;
}

export interface EngineStatus {
	ytdlp: DependencyInfo;
	ffmpeg: DependencyInfo;
}

export interface StartOptions {
	url: string;
	download_mode: DownloadMode;
	quality: string;
	format_id: string | null;
	playlist_items: string | null;
	output_dir: string | null;
}

export type ToolName = "audio_mp3" | "gif" | "compress";

export interface ToolRequest {
	tool: ToolName;
	input: string;
	output?: string | null;
	start_secs?: number | null;
	duration?: number | null;
	width?: number | null;
	fps?: number | null;
	crf?: number | null;
}

export interface ToolProgressEvent {
	tool: string;
	percent: number;
}

export interface ToolFinishedEvent {
	tool: string;
	success: boolean;
	output: string | null;
	message: string | null;
}

function messageOf(e: unknown): string {
	if (e instanceof Error) return e.message;
	if (typeof e === "string") return e;
	try {
		return JSON.stringify(e);
	} catch {
		return String(e);
	}
}

async function call<T>(fn: () => Promise<T>): Promise<T> {
	try {
		return await fn();
	} catch (e) {
		throw new Error(messageOf(e));
	}
}

export const api = {
	analyzeUrl: (url: string) => call(() => invoke<Analysis>("analyze_url", { url })),
	runTool: (request: ToolRequest) => call(() => invoke<string>("run_tool", { request })),
	startDownload: (options: StartOptions) =>
		call(() => invoke<number>("start_download", { options })),
	pauseDownload: (id: number) => call(() => invoke<void>("pause_download", { id })),
	resumeDownload: (id: number) => call(() => invoke<void>("resume_download", { id })),
	cancelDownload: (id: number) => call(() => invoke<void>("cancel_download", { id })),
	retryDownload: (id: number) => call(() => invoke<void>("retry_download", { id })),
	removeDownload: (id: number, deleteFile: boolean) =>
		call(() => invoke<void>("remove_download", { id, deleteFile })),
	clearFinished: () => call(() => invoke<void>("clear_finished")),
	pauseAll: () => call(() => invoke<void>("pause_all")),
	resumeAll: () => call(() => invoke<void>("resume_all")),
	queueSnapshot: () => call(() => invoke<QueueItemInfo[]>("queue_snapshot")),
	historyList: () => call(() => invoke<HistoryEntry[]>("history_list")),
	historyClear: () => call(() => invoke<void>("history_clear")),
	settingsGet: () => call(() => invoke<AppSettings>("settings_get")),
	settingsUpdate: (patch: Partial<AppSettings>) =>
		call(() => invoke<AppSettings>("settings_update", { patch })),
	settingsReset: () => call(() => invoke<AppSettings>("settings_reset")),
	engineStatus: () => call(() => invoke<EngineStatus>("engine_status")),
	engineInstall: (name: "ytdlp" | "ffmpeg") =>
		call(() => invoke<void>("engine_install", { name })),
	revealInFolder: (path: string) => call(() => invoke<void>("reveal_in_folder", { path })),
	openWithDefault: (path: string) => call(() => invoke<void>("open_with_default", { path })),
	defaultOutputDir: () => call(() => invoke<string>("default_output_dir"))
};

export function onQueueChanged(callback: (items: QueueItemInfo[]) => void): Promise<UnlistenFn> {
	return listen<QueueItemInfo[]>("queue-changed", (event) => callback(event.payload));
}

export function onToolProgress(callback: (e: ToolProgressEvent) => void): Promise<UnlistenFn> {
	return listen<ToolProgressEvent>("tool-progress", (event) => callback(event.payload));
}

export function onToolFinished(callback: (e: ToolFinishedEvent) => void): Promise<UnlistenFn> {
	return listen<ToolFinishedEvent>("tool-finished", (event) => callback(event.payload));
}

