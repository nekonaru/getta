<script lang="ts">
	import { onMount } from "svelte";
	import { open } from "@tauri-apps/plugin-dialog";
	import { api, onToolFinished, onToolProgress, type ToolName } from "#lib/api";
	import { toastStore } from "#lib/stores/toast.svelte";
	import { t } from "#lib/i18n.svelte";

	type ToolState = {
		input: string;
		running: boolean;
		progress: number;
		output: string | null;
		error: string | null;
	};
	type ParamSpec =
		| { kind: "select"; key: string; labelKey: string; options: Array<{ value: string; label: string }>; defaultValue: string }
		| { kind: "number"; key: string; labelKey: string; min: number; max: number; step: number; defaultValue: string }
		| { kind: "file"; key: string; labelKey: string; extensions: string[]; defaultValue: string };
	type ToolGroup = "audio" | "video" | "image" | "subtitles";
	type Icon = "audio" | "gif" | "compress" | "trim" | "convert" | "lossless" | "normalize" | "speed" | "video" | "resize" | "rotate" | "mute" | "frame" | "image" | "text" | "wave";

	const MEDIA = { name: "Media", extensions: ["mp4", "mkv", "webm", "mov", "avi", "m4a", "mp3", "wav", "flac", "ogg", "opus", "aac"] };
	const VIDEO = { name: "Video", extensions: ["mp4", "mkv", "webm", "mov", "avi"] };
	const IMAGE = { name: "Images", extensions: ["jpg", "jpeg", "png", "webp", "bmp", "gif", "avif"] };
	const SUBTITLES = { name: "Subtitles", extensions: ["srt", "vtt", "ass", "ssa"] };

	const TOOL_DEFS: Array<{
		id: ToolName;
		group: ToolGroup;
		name: () => string;
		desc: () => string;
		params: ParamSpec[];
		filters: { name: string; extensions: string[] };
		icon: Icon;
	}> = [
		{ id: "audio_mp3", group: "audio", name: () => t("tools.audioMp3"), desc: () => t("tools.audioMp3Desc"), params: [], filters: MEDIA, icon: "audio" },
		{ id: "audio_convert", group: "audio", name: () => t("tools.audioConvert"), desc: () => t("tools.audioConvertDesc"), params: [
			{ kind: "select", key: "format", labelKey: "tools.format", defaultValue: "mp3", options: [{ value: "mp3", label: "MP3" }, { value: "flac", label: "FLAC" }, { value: "ogg", label: "OGG" }, { value: "opus", label: "Opus" }, { value: "m4a", label: "M4A (AAC)" }, { value: "wav", label: "WAV" }] },
			{ kind: "number", key: "bitrate", labelKey: "tools.bitrate", min: 32, max: 320, step: 16, defaultValue: "192" },
		], filters: MEDIA, icon: "convert" },
		{ id: "audio_lossless", group: "audio", name: () => t("tools.audioLossless"), desc: () => t("tools.audioLosslessDesc"), params: [], filters: MEDIA, icon: "lossless" },
		{ id: "audio_normalize", group: "audio", name: () => t("tools.audioNormalize"), desc: () => t("tools.audioNormalizeDesc"), params: [], filters: MEDIA, icon: "normalize" },
		{ id: "audio_speed", group: "audio", name: () => t("tools.audioSpeed"), desc: () => t("tools.audioSpeedDesc"), params: [
			{ kind: "number", key: "speed", labelKey: "tools.speed", min: 0.5, max: 4, step: 0.25, defaultValue: "1" },
		], filters: MEDIA, icon: "speed" },
		{ id: "waveform_png", group: "audio", name: () => t("tools.waveform"), desc: () => t("tools.waveformDesc"), params: [], filters: MEDIA, icon: "wave" },
		{ id: "volume_boost", group: "audio", name: () => t("tools.volumeBoost"), desc: () => t("tools.volumeBoostDesc"), params: [
			{ kind: "select", key: "factor", labelKey: "tools.factor", defaultValue: "1.5", options: [{ value: "0.5", label: "50%" }, { value: "1", label: "100%" }, { value: "1.5", label: "150%" }, { value: "2", label: "200%" }, { value: "3", label: "300%" }] },
		], filters: MEDIA, icon: "speed" },
		{ id: "silence_trim", group: "audio", name: () => t("tools.silenceTrim"), desc: () => t("tools.silenceTrimDesc"), params: [], filters: MEDIA, icon: "trim" },
		{ id: "mp3_cover", group: "audio", name: () => t("tools.mp3Cover"), desc: () => t("tools.mp3CoverDesc"), params: [
			{ kind: "file", key: "cover", labelKey: "tools.pickCover", extensions: ["jpg", "jpeg", "png", "webp"], defaultValue: "" },
		], filters: MEDIA, icon: "image" },

		{ id: "trim", group: "video", name: () => t("tools.trim"), desc: () => t("tools.trimDesc"), params: [
			{ kind: "number", key: "start", labelKey: "tools.startAt", min: 0, max: 86400, step: 0.5, defaultValue: "0" },
			{ kind: "number", key: "duration", labelKey: "tools.duration", min: 0.5, max: 86400, step: 0.5, defaultValue: "10" },
		], filters: VIDEO, icon: "trim" },
		{ id: "gif", group: "video", name: () => t("tools.gif"), desc: () => t("tools.gifDesc"), params: [
			{ kind: "number", key: "start", labelKey: "tools.startAt", min: 0, max: 86400, step: 0.5, defaultValue: "0" },
			{ kind: "number", key: "duration", labelKey: "tools.duration", min: 0.2, max: 600, step: 0.5, defaultValue: "5" },
			{ kind: "number", key: "width", labelKey: "tools.width", min: 64, max: 1920, step: 10, defaultValue: "480" },
			{ kind: "number", key: "fps", labelKey: "tools.fps", min: 1, max: 50, step: 1, defaultValue: "10" },
		], filters: VIDEO, icon: "gif" },
		{ id: "compress", group: "video", name: () => t("tools.compress"), desc: () => t("tools.compressDesc"), params: [
			{ kind: "select", key: "crf", labelKey: "tools.crf", defaultValue: "30", options: [{ value: "23", label: "23 (high quality)" }, { value: "28", label: "28" }, { value: "30", label: "30 (balanced)" }, { value: "35", label: "35" }, { value: "40", label: "40 (tiny)" }] },
		], filters: VIDEO, icon: "compress" },
		{ id: "video_convert", group: "video", name: () => t("tools.videoConvert"), desc: () => t("tools.videoConvertDesc"), params: [
			{ kind: "select", key: "target", labelKey: "tools.target", defaultValue: "mp4", options: [{ value: "mp4", label: "MP4 (H.264)" }, { value: "mkv", label: "MKV" }, { value: "webm", label: "WebM (VP9)" }, { value: "mov", label: "MOV" }] },
			{ kind: "select", key: "mode", labelKey: "tools.mode", defaultValue: "remux", options: [{ value: "remux", label: t("tools.remux") }, { value: "encode", label: t("tools.encode") }] },
		], filters: VIDEO, icon: "video" },
		{ id: "video_resize", group: "video", name: () => t("tools.videoResize"), desc: () => t("tools.videoResizeDesc"), params: [
			{ kind: "select", key: "height", labelKey: "tools.height", defaultValue: "720", options: [{ value: "480", label: "480p" }, { value: "720", label: "720p" }, { value: "1080", label: "1080p" }, { value: "1440", label: "1440p" }, { value: "2160", label: "2160p" }] },
		], filters: VIDEO, icon: "resize" },
		{ id: "video_rotate", group: "video", name: () => t("tools.videoRotate"), desc: () => t("tools.videoRotateDesc"), params: [
			{ kind: "select", key: "angle", labelKey: "tools.angle", defaultValue: "90", options: [{ value: "90", label: "90°" }, { value: "180", label: "180°" }, { value: "270", label: "270°" }] },
		], filters: VIDEO, icon: "rotate" },
		{ id: "mute_video", group: "video", name: () => t("tools.muteVideo"), desc: () => t("tools.muteVideoDesc"), params: [], filters: VIDEO, icon: "mute" },
		{ id: "extract_frame", group: "video", name: () => t("tools.frame"), desc: () => t("tools.frameDesc"), params: [
			{ kind: "number", key: "time", labelKey: "tools.time", min: 0, max: 86400, step: 0.5, defaultValue: "0" },
			{ kind: "select", key: "target", labelKey: "tools.target", defaultValue: "jpg", options: [{ value: "jpg", label: "JPG" }, { value: "png", label: "PNG" }, { value: "webp", label: "WebP" }] },
		], filters: VIDEO, icon: "frame" },
		{ id: "contact_sheet", group: "video", name: () => t("tools.contactSheet"), desc: () => t("tools.contactSheetDesc"), params: [], filters: VIDEO, icon: "frame" },
		{ id: "square_crop", group: "video", name: () => t("tools.squareCrop"), desc: () => t("tools.squareCropDesc"), params: [], filters: VIDEO, icon: "resize" },
		{ id: "mux", group: "video", name: () => t("tools.mux"), desc: () => t("tools.muxDesc"), params: [
			{ kind: "file", key: "audio", labelKey: "tools.pickAudioTrack", extensions: ["mp3", "m4a", "wav", "flac", "ogg", "opus", "aac", "mka"], defaultValue: "" },
		], filters: VIDEO, icon: "convert" },

		{ id: "image_resize", group: "image", name: () => t("tools.imageResize"), desc: () => t("tools.imageResizeDesc"), params: [
			{ kind: "number", key: "width", labelKey: "tools.width", min: 16, max: 8192, step: 10, defaultValue: "1280" },
			{ kind: "select", key: "target", labelKey: "tools.target", defaultValue: "original", options: [{ value: "original", label: t("tools.original") }, { value: "jpg", label: "JPG" }, { value: "png", label: "PNG" }, { value: "webp", label: "WebP" }] },
		], filters: IMAGE, icon: "image" },
		{ id: "image_convert", group: "image", name: () => t("tools.imageConvert"), desc: () => t("tools.imageConvertDesc"), params: [
			{ kind: "select", key: "target", labelKey: "tools.target", defaultValue: "webp", options: [{ value: "jpg", label: "JPG" }, { value: "png", label: "PNG" }, { value: "webp", label: "WebP" }] },
		], filters: IMAGE, icon: "image" },

		{ id: "subtitle_convert", group: "subtitles", name: () => t("tools.subtitleConvert"), desc: () => t("tools.subtitleConvertDesc"), params: [
			{ kind: "select", key: "target", labelKey: "tools.target", defaultValue: "vtt", options: [{ value: "vtt", label: "WebVTT" }, { value: "srt", label: "SRT" }, { value: "ass", label: "ASS" }] },
		], filters: SUBTITLES, icon: "text" },
	];

	const GROUPS: Array<{ id: ToolGroup; label: () => string }> = [
		{ id: "audio", label: () => t("tools.groupAudio") },
		{ id: "video", label: () => t("tools.groupVideo") },
		{ id: "image", label: () => t("tools.groupImage") },
		{ id: "subtitles", label: () => t("tools.groupSubtitles") },
	];

	const TOOL_BY_ID = Object.fromEntries(TOOL_DEFS.map((d) => [d.id, d])) as Record<ToolName, (typeof TOOL_DEFS)[number]>;

	const freshState = (): ToolState => ({ input: "", running: false, progress: 0, output: null, error: null });

	let ffmpegReady = $state<boolean | null>(null);
	let expanded = $state<ToolName | null>("audio_mp3");
	let toolsState = $state<Record<ToolName, ToolState>>(
		Object.fromEntries(TOOL_DEFS.map((d) => [d.id, freshState()])) as Record<ToolName, ToolState>
	);
	let paramValues = $state<Record<ToolName, Record<string, string>>>(
		Object.fromEntries(
			TOOL_DEFS.map((d) => [d.id, Object.fromEntries(d.params.map((p) => [p.key, p.defaultValue]))])
		) as Record<ToolName, Record<string, string>>
	);

	onMount(() => {
		void api.engineStatus().then((engine) => (ffmpegReady = engine.ffmpeg.found)).catch(() => (ffmpegReady = null));
		const unlistenFinished = onToolFinished((event) => {
			const tool = event.tool as ToolName;
			const state = toolsState[tool];
			if (!state) return;
			state.running = false;
			if (event.success) {
				state.output = event.output;
				state.progress = 100;
				toastStore.push(t("tools.done"));
			} else {
				state.error = event.message ?? t("tools.failed");
				toastStore.push(state.error ?? t("tools.failed"), "error");
			}
		});
		const unlistenProgress = onToolProgress((event) => {
			const tool = event.tool as ToolName;
			if (toolsState[tool]) toolsState[tool].progress = event.percent;
		});
		return () => {
			void unlistenFinished.then((fn) => fn?.());
			void unlistenProgress.then((fn) => fn?.());
		};
	});

	function toolState(tool: ToolName): ToolState {
		return toolsState[tool];
	}

	async function pickFile(tool: ToolName) {
		try {
			const picked = await open({ multiple: false, directory: false, filters: [TOOL_BY_ID[tool].filters] });
			if (typeof picked === "string" && picked) {
				toolsState[tool].input = picked;
				toolsState[tool].output = null;
				toolsState[tool].error = null;
				toolsState[tool].progress = 0;
			}
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function pickParamFile(tool: ToolName, key: string, extensions: string[]) {
		try {
			const picked = await open({ multiple: false, directory: false, filters: [{ name: key, extensions }] });
			if (typeof picked === "string" && picked) {
				paramValues[tool][key] = picked;
			}
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function runTool(tool: ToolName) {
		const state = toolsState[tool];
		if (!state.input) return;
		state.running = true;
		state.output = null;
		state.error = null;
		state.progress = 0;
		try {
			await api.runTool({ tool, input: state.input, params: { ...paramValues[tool] } });
		} catch (e) {
			state.running = false;
			state.error = e instanceof Error ? e.message : String(e);
		}
	}

	async function reveal(path: string) {
		try {
			await api.revealInFolder(path);
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}
</script>

<div class="page">
	<div class="page-inner">
		<header class="page-head">
			<div>
				<p class="kicker">{t("tools.kicker")}</p>
				<h1 class="page-title">{t("tools.title")}</h1>
			</div>
		</header>
		<p class="intro muted">{t("tools.intro")}</p>

		{#if ffmpegReady === false}
			<div class="engine-banner" role="alert">
				<span>{t("tools.ffmpegMissing")}</span>
				<a class="btn btn-accent btn-sm" href="/settings">{t("tools.goEngine")}</a>
			</div>
		{/if}

		<div class="tool-list">
			{#each GROUPS as group (group.id)}
				<p class="group-label">{group.label()}</p>
				{#each TOOL_DEFS.filter((d) => d.group === group.id) as def (def.id)}
					{@const state = toolState(def.id)}
					<section class="tool card" class:open={expanded === def.id}>
						<button class="tool-head" aria-expanded={expanded === def.id} onclick={() => (expanded = expanded === def.id ? null : def.id)}>
							<span class="tool-icon" aria-hidden="true">
								{#if def.icon === "audio"}
									<svg viewBox="0 0 24 24"><path d="M9 6.5v11l9-5.5z"/><circle cx="6.5" cy="17.5" r="2.5"/></svg>
								{:else if def.icon === "gif"}
									<svg viewBox="0 0 24 24"><rect x="3" y="5" width="18" height="14"/><path d="M8 12v.01M12 12v.01M16 12v.01"/></svg>
								{:else if def.icon === "compress"}
									<svg viewBox="0 0 24 24"><path d="M4 12h6l-2-5 4 10 2-5h6"/></svg>
								{:else if def.icon === "trim"}
									<svg viewBox="0 0 24 24"><path d="M6 8V6m0 12v-2M18 8V6m0 12v-2M4 10h6M4 14h6M14 10h6M14 14h6"/></svg>
								{:else if def.icon === "video"}
									<svg viewBox="0 0 24 24"><path d="M4 6h12M4 12h12M4 18h12"/><path d="M19 8h3M19 16h3"/></svg>
								{:else if def.icon === "resize"}
									<svg viewBox="0 0 24 24"><path d="M9 4H4v5M15 4h5v5M9 20H4v-5M15 20h5v-5"/></svg>
								{:else if def.icon === "rotate"}
									<svg viewBox="0 0 24 24"><path d="M20 11A8 8 0 1 0 18 17"/><path d="M20 5v6h-6"/></svg>
								{:else if def.icon === "mute"}
									<svg viewBox="0 0 24 24"><path d="M11 6 5 10H3v4h2l6 4z"/><path d="m17 9 4 4m0-4-4 4"/></svg>
								{:else if def.icon === "frame"}
									<svg viewBox="0 0 24 24"><rect x="3" y="6" width="18" height="12" rx="2"/><circle cx="9" cy="10" r="1.4"/><path d="m5 18 6-6 4 4 3-3 6 6"/></svg>
								{:else if def.icon === "convert"}
									<svg viewBox="0 0 24 24"><path d="M7 8h11l-3-3M17 16H6l3 3"/></svg>
								{:else if def.icon === "lossless"}
									<svg viewBox="0 0 24 24"><rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"/></svg>
								{:else if def.icon === "normalize"}
									<svg viewBox="0 0 24 24"><path d="M4 12h2M9 8v8M13 6v12M17 9v6M21 12h-1"/></svg>
								{:else if def.icon === "speed"}
									<svg viewBox="0 0 24 24"><path d="M5 6l6 6-6 6M13 6l6 6-6 6"/></svg>
								{:else if def.icon === "wave"}
									<svg viewBox="0 0 24 24"><path d="M3 12c2-4 4-4 6 0s4 4 6 0 4-4 6 0"/></svg>
								{:else if def.icon === "image"}
									<svg viewBox="0 0 24 24"><rect x="3" y="5" width="18" height="14" rx="2"/><circle cx="9" cy="10" r="1.4"/><path d="m5 17 6-6 5 5 3-3 5 5"/></svg>
								{:else}
									<svg viewBox="0 0 24 24"><path d="M5 7h14M5 12h14M5 17h9"/></svg>
								{/if}
							</span>
							<span class="tool-text">
								<span class="tool-name">{def.name()}</span>
								<span class="tool-desc muted">{def.desc()}</span>
							</span>
							<svg class="chevron" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 9l6 6 6-6"/></svg>
						</button>

						{#if expanded === def.id}
							<div class="tool-body">
								<div class="file-row">
									<button class="btn btn-sm" onclick={() => pickFile(def.id)} disabled={state.running}>{t("tools.pickFile")}</button>
									<span class="file-name mono" title={state.input}>{state.input ? state.input.split(/[\\/]/).pop() : "-"}</span>
								</div>

								{#if def.params.length > 0}
									<div class="options-grid">
										{#each def.params as spec (spec.key)}
											{#if spec.kind === "select"}
												<label><span class="field-label">{t(spec.labelKey)}</span>
													<select class="input" bind:value={paramValues[def.id][spec.key]} disabled={state.running}>
														{#each spec.options as opt (opt.value)}
															<option value={opt.value}>{opt.label}</option>
														{/each}
													</select>
												</label>
											{:else if spec.kind === "number"}
												<label><span class="field-label">{t(spec.labelKey)}</span>
													<input class="input" type="number" min={spec.min} max={spec.max} step={spec.step} bind:value={paramValues[def.id][spec.key]} disabled={state.running} />
												</label>
											{:else}
												<div class="file-param">
													<span class="field-label">{t(spec.labelKey)}</span>
													<div class="file-param-row">
														<button class="btn btn-sm" onclick={() => pickParamFile(def.id, spec.key, spec.extensions)} disabled={state.running}>{t("tools.pickFile")}</button>
														<span class="file-name mono" title={paramValues[def.id][spec.key]}>{paramValues[def.id][spec.key] ? paramValues[def.id][spec.key].split(/[\\/]/).pop() : t("tools.noFileChosen")}</span>
													</div>
												</div>
											{/if}
										{/each}
									</div>
								{/if}

								<div class="run-row">
									<button class="btn btn-accent" disabled={!state.input || state.running || ffmpegReady === false} onclick={() => runTool(def.id)}>
										{#if state.running}<span class="spinner"></span>{t("tools.running")}{:else}{t("tools.run")}{/if}
									</button>
									{#if state.output}
										<button class="btn btn-sm btn-ghost" onclick={() => reveal(state.output ?? "")}>{t("tools.reveal")}</button>
									{/if}
								</div>
								{#if state.running || state.progress > 0}
									<div class="progress"><div class="progress-fill progress-sheen" style="width: {state.progress}%"></div></div>
								{/if}
								{#if state.error}
									<p class="error-box">{state.error}</p>
								{/if}
								{#if state.output}
									<p class="output-line"><span class="field-label" style="margin:0">{t("tools.output")}</span><code class="mono output-path" title={state.output}>{state.output}</code></p>
								{/if}
							</div>
						{/if}
					</section>
				{/each}
			{/each}
		</div>
	</div>
</div>

<style>
	.intro {
		margin: 0 0 14px;
		font-size: 13px;
	}

	.engine-banner {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		background: var(--warn-soft);
		border: 1px solid var(--warn);
		color: var(--warn);
		border-radius: var(--r-sm);
		padding: 10px 14px;
		font-size: 12.5px;
		font-weight: 600;
		margin-bottom: 14px;
	}

	.tool-list {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.group-label {
		font-size: 10.5px;
		font-weight: 700;
		letter-spacing: 0.12em;
		text-transform: uppercase;
		color: var(--muted);
		margin: 8px 4px -4px;
	}

	.tool {
		overflow: hidden;
	}

	.tool-head {
		display: grid;
		grid-template-columns: 52px 1fr auto;
		align-items: center;
		gap: 14px;
		width: 100%;
		border: none;
		background: transparent;
		padding: 16px 18px;
		cursor: pointer;
		text-align: left;
		color: inherit;
	}

	.tool-head:hover {
		background: var(--card-2);
	}

	.tool-icon {
		width: 52px;
		height: 52px;
		background: var(--accent-faint);
		border: 1px solid var(--border-soft);
		border-radius: var(--r-md);
		display: grid;
		place-items: center;
		color: var(--accent);
	}

	.tool-icon svg {
		width: 26px;
		height: 26px;
		stroke: currentColor;
		fill: none;
		stroke-width: 1.6;
		stroke-linecap: round;
		stroke-linejoin: round;
	}

	.tool-text {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}

	.tool-name {
		font-weight: 680;
		font-size: 15px;
	}

	.tool-desc {
		font-size: 12px;
	}

	.chevron {
		width: 18px;
		height: 18px;
		stroke: var(--muted);
		fill: none;
		stroke-width: 1.6;
		stroke-linecap: round;
		stroke-linejoin: round;
		transition: transform 0.2s ease;
	}

	.tool.open .chevron {
		transform: rotate(180deg);
	}

	.tool-body {
		display: flex;
		flex-direction: column;
		gap: 12px;
		border-top: 1px solid var(--border);
		padding: 16px 18px 18px;
	}

	.file-row {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.file-param {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.file-param-row {
		display: flex;
		align-items: center;
		gap: 10px;
		min-width: 0;
	}

	.file-name {
		font-size: 11px;
		color: var(--muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.options-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
		gap: 12px;
	}

	.options-grid label {
		display: flex;
		flex-direction: column;
	}

	.run-row {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.output-line {
		display: flex;
		align-items: center;
		gap: 10px;
		margin: 0;
	}

	.output-path {
		font-size: 10.5px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.error-box {
		font-size: 12px;
		color: var(--bad);
		background: var(--bad-soft);
		padding: 8px 10px;
		border-radius: var(--r-sm);
		margin: 0;
	}
</style>
