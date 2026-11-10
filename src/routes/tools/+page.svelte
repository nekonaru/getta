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

	let ffmpegReady = $state<boolean | null>(null);
	let expanded = $state<ToolName | null>("audio_mp3");
	let gifStart = $state("0");
	let gifDuration = $state("5");
	let gifWidth = $state("480");
	let gifFps = $state("10");
	let trimStart = $state("0");
	let trimDuration = $state("10");
	let compressCrf = $state("30");

	let toolsState = $state<Record<ToolName, ToolState>>({
		audio_mp3: { input: "", running: false, progress: 0, output: null, error: null },
		gif: { input: "", running: false, progress: 0, output: null, error: null },
		compress: { input: "", running: false, progress: 0, output: null, error: null },
		trim: { input: "", running: false, progress: 0, output: null, error: null }
	});

	onMount(() => {
		void api.engineStatus().then((engine) => (ffmpegReady = engine.ffmpeg.found)).catch(() => (ffmpegReady = null));
		const unlistenFinished = onToolFinished((event) => {
			const tool = event.tool as ToolName;
			toolsState[tool].running = false;
			if (event.success) {
				toolsState[tool].output = event.output;
				toolsState[tool].progress = 100;
				toastStore.push(t("tools.done"));
			} else {
				toolsState[tool].error = event.message ?? t("tools.failed");
				toastStore.push(toolsState[tool].error ?? t("tools.failed"), "error");
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
			const picked = await open({ multiple: false, directory: false, filters: [{ name: "Media", extensions: ["mp4", "mkv", "webm", "mov", "avi", "m4a", "mp3", "wav", "flac", "ogg"] }] });
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

	async function runTool(tool: ToolName) {
		const state = toolsState[tool];
		if (!state.input) return;
		state.running = true;
		state.output = null;
		state.error = null;
		state.progress = 0;
		try {
			const request =
				tool === "gif"
					? {
							tool,
							input: state.input,
							start_secs: Number(gifStart) || 0,
							duration: Number(gifDuration) || 5,
							width: Number(gifWidth) || 480,
							fps: Number(gifFps) || 10
						}
					: tool === "trim"
						? {
								tool,
								input: state.input,
								start_secs: Number(trimStart) || 0,
								duration: Number(trimDuration) || 10
							}
						: tool === "compress"
							? { tool, input: state.input, crf: Number(compressCrf) || 30 }
							: { tool, input: state.input };
			await api.runTool(request);
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

	const TOOL_DEFS: Array<{
		id: ToolName;
		name: () => string;
		desc: () => string;
	}> = [
		{
			id: "audio_mp3",
			name: () => t("tools.audioMp3"),
			desc: () => t("tools.audioMp3Desc")
		},
		{
			id: "gif",
			name: () => t("tools.gif"),
			desc: () => t("tools.gifDesc")
		},
		{
			id: "trim",
			name: () => t("tools.trim"),
			desc: () => t("tools.trimDesc")
		},
		{
			id: "compress",
			name: () => t("tools.compress"),
			desc: () => t("tools.compressDesc")
		}
	];
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
			{#each TOOL_DEFS as def (def.id)}
				{@const state = toolState(def.id)}
				<section class="tool card" class:open={expanded === def.id}>
					<button class="tool-head" aria-expanded={expanded === def.id} onclick={() => (expanded = expanded === def.id ? null : def.id)}>
						<span class="tool-icon" aria-hidden="true">
							{#if def.id === "audio_mp3"}
								<svg viewBox="0 0 24 24"><path d="M9 6.5v11l9-5.5z"/><circle cx="6.5" cy="17.5" r="2.5"/></svg>
							{:else if def.id === "gif"}
								<svg viewBox="0 0 24 24"><rect x="3" y="5" width="18" height="14"/><path d="M8 12v.01M12 12v.01M16 12v.01"/></svg>
							{:else if def.id === "compress"}
								<svg viewBox="0 0 24 24"><path d="M4 12h6l-2-5 4 10 2-5h6"/></svg>
							{:else}
								<svg viewBox="0 0 24 24"><path d="M6 8V6m0 12v-2M18 8V6m0 12v-2M4 10h6M4 14h6M14 10h6M14 14h6"/></svg>
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

							{#if def.id === "gif"}
								<div class="options-grid">
									<label><span class="field-label">{t("tools.startAt")}</span><input class="input" type="number" min="0" step="0.5" bind:value={gifStart} disabled={state.running} /></label>
									<label><span class="field-label">{t("tools.duration")}</span><input class="input" type="number" min="0.2" step="0.5" bind:value={gifDuration} disabled={state.running} /></label>
									<label><span class="field-label">{t("tools.width")}</span><input class="input" type="number" min="64" max="1920" step="10" bind:value={gifWidth} disabled={state.running} /></label>
									<label><span class="field-label">{t("tools.fps")}</span><input class="input" type="number" min="1" max="50" step="1" bind:value={gifFps} disabled={state.running} /></label>
								</div>
							{:else if def.id === "trim"}
								<div class="options-grid">
									<label><span class="field-label">{t("tools.startAt")}</span><input class="input" type="number" min="0" step="0.5" bind:value={trimStart} disabled={state.running} /></label>
									<label><span class="field-label">{t("tools.duration")}</span><input class="input" type="number" min="0.5" step="0.5" bind:value={trimDuration} disabled={state.running} /></label>
								</div>
							{:else if def.id === "compress"}
								<div class="options-grid">
									<label><span class="field-label">{t("tools.crf")}</span>
										<select class="input" bind:value={compressCrf} disabled={state.running}>
											<option value="23">23 (high quality)</option>
											<option value="28">28</option>
											<option value="30">30 (balanced)</option>
											<option value="35">35</option>
											<option value="40">40 (tiny)</option>
										</select>
									</label>
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
