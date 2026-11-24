<script lang="ts">
	import LogoMark from "#lib/components/LogoMark.svelte";
	import { t } from "#lib/i18n.svelte";
	import { openExternal } from "#lib/external";
	import { api, type Diagnostics } from "#lib/api";
	import { toastStore } from "#lib/stores/toast.svelte";
	import { onMount } from "svelte";
	import { check } from "@tauri-apps/plugin-updater";
	import { relaunch } from "@tauri-apps/plugin-process";
	import pkg from "../../../package.json";

	const version = pkg.version;
	type UpdateState = "idle" | "checking" | "uptodate" | "available" | "installing" | "error";
	let updateState = $state<UpdateState>("idle");
	let updateVersion = $state<string | null>(null);
	let updateError = $state<string | null>(null);
	let diagnostics = $state<Diagnostics | null>(null);
	let checkingEngines = $state(false);

	onMount(async () => {
		try {
			diagnostics = await api.getDiagnostics();
		} catch {
			diagnostics = null;
		}
	});

	async function verifyEngines() {
		checkingEngines = true;
		try {
			const status = await api.engineStatus();
			const ok = status.ytdlp.found && status.ffmpeg.found;
			toastStore.push(
				ok
					? t("about.enginesOk", { ytdlp: status.ytdlp.version ?? "?", ffmpeg: status.ffmpeg.version ?? "?" })
					: t("about.enginesMissing"),
				ok ? "info" : "error"
			);
			diagnostics = await api.getDiagnostics();
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		} finally {
			checkingEngines = false;
		}
	}

	async function openDataFolder() {
		if (!diagnostics) return;
		try {
			await api.revealInFolder(diagnostics.data_dir);
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function checkForUpdates() {
		updateState = "checking";
		updateError = null;
		try {
			const update = await check();
			if (update) {
				updateVersion = update.version;
				updateState = "available";
			} else {
				updateState = "uptodate";
			}
		} catch (e) {
			updateError = e instanceof Error ? e.message : String(e);
			updateState = "error";
		}
	}

	async function installUpdate() {
		updateState = "installing";
		updateError = null;
		try {
			const update = await check();
			if (!update) {
				updateState = "uptodate";
				return;
			}
			await update.downloadAndInstall();
			await relaunch();
		} catch (e) {
			updateError = e instanceof Error ? e.message : String(e);
			updateState = "error";
		}
	}
</script>

<div class="page">
	<div class="page-inner about-inner">
		<div class="about-mark" aria-hidden="true">
			<LogoMark size={72} />
		</div>
		<h1 class="about-name">Getta</h1>
		<p class="about-tagline">{t("about.tagline")}</p>

		<div class="card about-card">
			<p>{t("about.body")}</p>
			<p class="muted">{t("about.engine")}</p>

			<dl class="about-meta">
				<div><dt>Version</dt><dd class="mono">v{version}</dd></div>
				<div><dt>Identifier</dt><dd class="mono">com.getta.app</dd></div>
				<div><dt>License</dt><dd>MIT</dd></div>
			</dl>

			<nav class="about-links">
				<button class="text-link" onclick={() => openExternal("https://github.com/nekonaru/getta")}>{t("about.source")}</button>
				<button class="text-link" onclick={() => openExternal("https://github.com/nekonaru/getta/releases")}>{t("about.releases")}</button>
				<button class="text-link" onclick={() => openExternal("https://github.com/nekonaru/getta/issues/new")}>{t("about.report")}</button>
				<a class="text-link" href="/support">{t("about.support")}</a>
			</nav>
			<div class="update-block">
				<p class="update-title">{t("about.update")}</p>
				{#if updateState === "idle"}
					<button class="btn btn-sm" onclick={checkForUpdates}>{t("about.updateCheck")}</button>
				{:else if updateState === "checking"}
					<p class="muted">{t("about.updateChecking")}</p>
				{:else if updateState === "uptodate"}
					<p class="muted">{t("about.updateUpToDate")}</p>
				{:else if updateState === "available"}
					<div class="update-row">
						<p class="muted">{t("about.updateAvailable", { version: updateVersion ?? "" })}</p>
						<button class="btn btn-accent btn-sm" onclick={installUpdate}>{t("about.updateInstall")}</button>
					</div>
				{:else if updateState === "installing"}
					<p class="muted"><span class="spinner"></span> {t("about.updateInstalling")}</p>
				{:else if updateState === "error"}
					<p class="error-box">{t("about.updateError", { error: updateError ?? "unknown" })}</p>
					<button class="btn btn-sm" onclick={checkForUpdates}>{t("about.updateCheck")}</button>
				{/if}
			</div>
			{#if diagnostics}
				<div class="diagnostics">
					<p class="update-title">{t("about.diagnostics")}</p>
					<dl class="diag-list">
						<div><dt>{t("about.dataDir")}</dt><dd class="mono" title={diagnostics.data_dir}>{diagnostics.data_dir}</dd></div>
						<div><dt>{t("about.outputDir")}</dt><dd class="mono" title={diagnostics.output_dir}>{diagnostics.output_dir}</dd></div>
						<div><dt>{t("about.historyEntries")}</dt><dd class="mono">{diagnostics.history_count}</dd></div>
						<div><dt>{t("about.followedChannels")}</dt><dd class="mono">{diagnostics.follows_count}</dd></div>
						<div><dt>{t("about.pendingRecovery")}</dt><dd class="mono">{diagnostics.recovery_count}</dd></div>
					</dl>
					<div class="diag-actions">
						<button class="btn btn-sm btn-ghost" onclick={verifyEngines} disabled={checkingEngines}>{checkingEngines ? t("about.updateChecking") : t("about.verifyEngines")}</button>
						<button class="btn btn-sm btn-ghost" onclick={openDataFolder}>{t("about.openData")}</button>
					</div>
				</div>
			{/if}
			<p class="muted license-line">{t("about.license")}</p>
		</div>
	</div>
</div>

<style>
	.about-inner {
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		gap: 6px;
		padding-top: clamp(32px, 10vh, 72px);
	}

	.about-mark {
		width: 104px;
		height: 104px;
		border-radius: 28px;
		background: var(--accent);
		display: grid;
		place-items: center;
		color: var(--on-accent);
		margin-bottom: 10px;
	}

	.about-name {
		font-family: var(--font-display);
		font-size: 2.6rem;
		font-weight: 800;
		letter-spacing: -0.02em;
	}

	.about-tagline {
		color: var(--muted);
		margin: 0 0 12px;
	}

	.about-card {
		padding: 22px 26px;
		max-width: 560px;
		text-align: left;
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.about-card p {
		margin: 0;
	}

	.about-meta {
		display: flex;
		gap: 28px;
		margin: 0;
		flex-wrap: wrap;
		border-top: 1px solid var(--border);
		padding-top: 14px;
	}

	.about-meta dt {
		font-size: 11px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.07em;
		color: var(--muted);
		margin-bottom: 2px;
	}

	.about-meta dd {
		margin: 0;
		font-size: 13px;
	}

	.about-links {
		display: flex;
		gap: 16px;
		flex-wrap: wrap;
	}

	.about-links .text-link {
		color: var(--accent);
		font-weight: 600;
		font-size: 13.5px;
		text-decoration: none;
		background: transparent;
		border: none;
		padding: 0;
		cursor: pointer;
		font-family: var(--font-body);
	}

	.about-links .text-link:hover {
		text-decoration: underline;
	}

	.update-block {
		border-top: 1px solid var(--border);
		padding-top: 14px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.update-title {
		font-size: 11px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.07em;
		color: var(--muted);
		margin: 0;
	}

	.update-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		flex-wrap: wrap;
	}

	.diagnostics {
		border-top: 1px solid var(--border);
		padding-top: 14px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.diag-list {
		display: grid;
		grid-template-columns: minmax(110px, auto) minmax(0, 1fr);
		gap: 4px 12px;
		margin: 0;
		font-size: 11px;
	}

	.diag-list dt {
		color: var(--muted);
		font-weight: 600;
	}

	.diag-list dd {
		margin: 0;
		min-width: 0;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.diag-actions {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
	}

	.license-line {
		font-size: 12px;
	}
</style>


