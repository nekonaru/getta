import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type DownloadMode = "auto" | "audio" | "mute" | "subs";
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
	scheduled_at?: number | null;
	priority?: number;
	speed_limit_mbps?: number;
	group_id?: string | null;
	after_id?: number | null;
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
	tags?: string[];
	rating?: number;
	note?: string;
	favorite?: boolean;
	watch_later?: boolean;
}

export interface FollowChannel {
	id: string;
	url: string;
	title: string;
	added_at: number;
	last_checked_at: number | null;
	auto_download: boolean;
	seen_ids: string[];
	check_interval_secs?: number | null;
}

export interface RecoveryEntry {
	id: number;
	url: string;
	title: string;
	download_mode: string;
	quality: string;
	format_id: string | null;
	playlist_items: string | null;
	output_dir: string | null;
}

export interface CookieBucket {
	id: string;
	name: string;
	file: string;
	domain: string;
	added_at: number;
}

export interface DownloadRule {
	id: string;
	pattern: string;
	dir: string;
	quality: string;
	mode: string;
}

export interface AppSettings {
	output_dir: string;
	always_ask_path: boolean;
	organize_by_platform: boolean;
	skip_existing: boolean;
	download_archive: boolean;
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
	split_by_chapters: boolean;
	live_from_start: boolean;
	theme: "system" | "light" | "dark";
	language: "en" | "id";
	reduce_motion: boolean;
	autostart: boolean;
	onboarding_completed: boolean;
	close_to_tray: boolean;
	cookies_from_browser: string;
	cookie_file: string;
	proxy: string;
	notifications: boolean;
	prevent_sleep: boolean;
	ytdlp_path: string | null;
	ffmpeg_path: string | null;
	download_rules: DownloadRule[];
	cookie_buckets: CookieBucket[];
	cookie_trash: CookieBucket[];
	presets: DownloadPreset[];
	smart_folders: SmartFolder[];
	retry_delay_secs: number;
	retry_multiplier: number;
	quiet_hours_enabled: boolean;
	quiet_start_hour: number;
	quiet_end_hour: number;
	post_action: string;
	post_action_dir: string;
	webhook_url: string;
	webhook_events: string;
	api_enabled: boolean;
	api_token: string;
	watch_folder: string;
	clipboard_monitor: boolean;
	resolve_shortlinks: boolean;
	accent: string;
	compact_mode: boolean;
	shortcuts: Record<string, string>;
	rss_feeds: RssFeed[];
	active_profile: string;
	safe_mode: boolean;
	automation_errors: number;
	retry_storm_cap: number;
	disk_guard_gb: number;
}

export interface RssFeed {
	id: string;
	url: string;
	title: string;
	seen: string[];
	last_checked_at: number;
}

export interface DownloadPreset {
	id: string;
	name: string;
	quality: string;
	mode: string;
	dir: string;
	template: string;
	priority: number;
	speed_limit_mbps: number;
}

export interface SmartFolder {
	id: string;
	name: string;
	query: string;
	kind: string;
}

export interface CookieReport {
	ok: boolean;
	total: number;
	expired: number;
	sites: string[];
	soonest_expiry: number | null;
	error: string | null;
}

export interface Diagnostics {
	data_dir: string;
	settings_file: string;
	output_dir: string;
	history_count: number;
	follows_count: number;
	recovery_count: number;
	autostart: boolean;
	close_to_tray: boolean;
	proxy_configured: boolean;
	cookies_configured: boolean;
}

export interface HealthReport {
	data_dir: string;
	output_dir: string;
	disk_free_bytes: number;
	disk_total_bytes: number;
	enough_space: boolean;
	output_writable: boolean;
	history_count: number;
	audit_count: number;
}

export interface ChapterMarker {
	title: string | null;
	start_seconds: number;
	end_seconds: number;
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
	start_at?: number | null;
	priority?: number;
	speed_limit_mbps?: number;
	expected_sha256?: string | null;
	group_id?: string | null;
	after_id?: number | null;
	preset_id?: string | null;
	direct?: boolean;
}

export type ToolName =
	| "audio_mp3"
	| "gif"
	| "compress"
	| "trim"
	| "audio_convert"
	| "audio_lossless"
	| "audio_normalize"
	| "audio_speed"
	| "video_convert"
	| "video_resize"
	| "video_rotate"
	| "mute_video"
	| "extract_frame"
	| "subtitle_convert"
	| "image_resize"
	| "image_convert"
	| "waveform_png"
	| "volume_boost"
	| "silence_trim"
	| "square_crop"
	| "mux"
	| "mp3_cover"
	| "contact_sheet";

export interface ToolRequest {
	tool: ToolName;
	input: string;
	output?: string | null;
	params?: Record<string, string>;
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
	clearSchedule: (id: number) => call(() => invoke<void>("clear_schedule", { id })),
	removeDownload: (id: number, deleteFile: boolean) =>
		call(() => invoke<void>("remove_download", { id, deleteFile })),
	clearFinished: () => call(() => invoke<void>("clear_finished")),
	pauseAll: () => call(() => invoke<void>("pause_all")),
	resumeAll: () => call(() => invoke<void>("resume_all")),
	queueSnapshot: () => call(() => invoke<QueueItemInfo[]>("queue_snapshot")),
	takePendingLinks: () => call(() => invoke<string[]>("take_pending_links")),
	importLinksFile: (path: string) => call(() => invoke<number>("import_links_file", { path })),
	moveItem: (id: number, delta: number) => call(() => invoke<void>("move_item", { id, delta })),
	retryAllFailed: () => call(() => invoke<number>("retry_all_failed")),
	moveItemTo: (id: number, targetIndex: number) =>
		call(() => invoke<void>("move_item_to", { id, targetIndex })),
	getRecoveryItems: () => call(() => invoke<RecoveryEntry[]>("get_recovery_items")),
	restoreRecovery: () => call(() => invoke<number>("restore_recovery")),
	dismissRecovery: () => call(() => invoke<void>("dismiss_recovery")),
	followList: () => call(() => invoke<FollowChannel[]>("follow_list")),
	followAdd: (url: string) => call(() => invoke<FollowChannel>("follow_add", { url })),
	followRemove: (id: string) => call(() => invoke<boolean>("follow_remove", { id })),
	followSetAuto: (id: string, auto: boolean) => call(() => invoke<boolean>("follow_set_auto", { id, auto })),
	followCheckNow: (id: string) => call(() => invoke<number>("follow_check_now", { id })),
	followSetInterval: (id: string, secs: number | null) =>
		call(() => invoke<boolean>("follow_set_interval", { id, secs })),
	followCheckAll: () => call(() => invoke<number>("follow_check_all")),
	deleteFiles: (paths: string[]) => call(() => invoke<string[]>("delete_files", { paths })),
	getDiagnostics: () => call(() => invoke<Diagnostics>("get_diagnostics")),
	exportQueue: (path: string) => call(() => invoke<void>("export_queue", { path })),
	importQueue: (path: string) => call(() => invoke<number>("import_queue", { path })),
	getClipboardText: () => call(() => invoke<string>("get_clipboard_text")),
	setClipboardText: (text: string) => call(() => invoke<void>("set_clipboard_text", { text })),
	historyList: () => call(() => invoke<HistoryEntry[]>("history_list")),
	historyClear: () => call(() => invoke<void>("history_clear")),
	historyRemove: (id: string) => call(() => invoke<boolean>("history_remove", { id })),
	exportHistoryCsv: (path: string) => call(() => invoke<void>("export_history_csv", { path })),
	exportHistoryJson: (path: string) => call(() => invoke<void>("export_history_json", { path })),
	cookieBucketCheck: (path: string) => call(() => invoke<CookieReport>("cookie_bucket_check", { path })),
	probeChapters: (path: string) => call(() => invoke<ChapterMarker[]>("probe_chapters", { path })),
	findDuplicates: () => call(() => invoke<string[][]>("find_duplicates")),
	settingsGet: () => call(() => invoke<AppSettings>("settings_get")),
	settingsUpdate: (patch: Partial<AppSettings>) =>
		call(() => invoke<AppSettings>("settings_update", { patch })),
	settingsReset: () => call(() => invoke<AppSettings>("settings_reset")),
	engineStatus: () => call(() => invoke<EngineStatus>("engine_status")),
	engineInstall: (name: "ytdlp" | "ffmpeg") =>
		call(() => invoke<void>("engine_install", { name })),
	revealInFolder: (path: string) => call(() => invoke<void>("reveal_in_folder", { path })),
	openWithDefault: (path: string) => call(() => invoke<void>("open_with_default", { path })),
	defaultOutputDir: () => call(() => invoke<string>("default_output_dir")),
	historyPatch: (id: string, patch: { tags?: string[]; rating?: number; note?: string; favorite?: boolean; watch_later?: boolean }) =>
		call(() => invoke<boolean>("history_patch", { id, ...patch })),
	batchRename: (ids: string[], template: string) =>
		call(() => invoke<number>("batch_rename", { ids, template })),
	getHealth: (neededBytes: number) =>
		call(() => invoke<HealthReport>("get_health", { neededBytes })),
	readAudit: () => call(() => invoke<string[]>("read_audit")),
	exportLibraryHtml: (path: string) =>
		call(() => invoke<void>("export_library_html", { path })),
	createBackup: (path: string) => call(() => invoke<void>("create_backup", { path })),
	restoreBackup: (path: string) => call(() => invoke<number>("restore_backup", { path })),
	listProfiles: () => call(() => invoke<string[]>("list_profiles")),
	saveProfile: (name: string) => call(() => invoke<void>("save_profile", { name })),
	loadProfile: (name: string) => call(() => invoke<AppSettings>("load_profile", { name })),
	historyRestore: (entry: HistoryEntry) => call(() => invoke<void>("history_restore", { entry })),
	resetSafeMode: () => call(() => invoke<AppSettings>("reset_safe_mode")),
	createDiagnosticBundle: (path: string) => call(() => invoke<void>("create_diagnostic_bundle", { path }))
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

