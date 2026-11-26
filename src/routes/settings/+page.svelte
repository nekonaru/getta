<script lang="ts">
	import { onMount } from "svelte";
	import { open } from "@tauri-apps/plugin-dialog";
	import { isEnabled, enable, disable } from "@tauri-apps/plugin-autostart";
	import { api, type EngineStatus, type DownloadRule, type CookieBucket, type AppSettings } from "#lib/api";
	import { settingsStore } from "#lib/stores/settings.svelte";
	import { toastStore } from "#lib/stores/toast.svelte";
	import { t } from "#lib/i18n.svelte";

	type Category = "output" | "transfer" | "appearance" | "engine" | "automation" | "data";
	let category = $state<Category>("output");
	let engine = $state<EngineStatus | null>(null);
	let installBusy = $state<string | null>(null);
	let patching = $state(false);
	let inFlight = 0;
	let updateChain = Promise.resolve();
	let autostartEnabled = $state(false);
	let auditLines = $state<string[]>([]);

	function beginPatch() {
		inFlight += 1;
		patching = true;
	}

	function endPatch() {
		inFlight = Math.max(0, inFlight - 1);
		patching = inFlight > 0;
	}

	const settings = $derived(settingsStore.value);

	onMount(async () => {
		await refreshEngine();
		try {
			autostartEnabled = await isEnabled();
		} catch {
			autostartEnabled = settingsStore.value?.autostart ?? false;
		}
	});

	async function toggleAutostart() {
		const next = !autostartEnabled;
		try {
			if (next) await enable();
			else await disable();
			await settingsStore.update({ autostart: next });
			autostartEnabled = next;
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function refreshEngine() {
		try {
			engine = await api.engineStatus();
		} catch {
			engine = null;
		}
	}

	async function patch(p: Partial<AppSettings>): Promise<boolean> {
		beginPatch();
		try {
			await settingsStore.update(p);
			return true;
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
			return false;
		} finally {
			endPatch();
		}
	}

	/// Serialise updates that are computed from the current settings so a
	/// rapid second click cannot overwrite the first with a stale snapshot.
	function enqueueUpdate(updater: (current: AppSettings | null) => Partial<AppSettings>): Promise<void> {
		const task = updateChain.then(async () => {
			await patch(updater(settingsStore.value));
		});
		updateChain = task.catch(() => {});
		return task;
	}

	async function pickFolder() {
		try {
			const picked = await open({ directory: true, multiple: false });
			if (typeof picked === "string" && picked) {
				await patch({ output_dir: picked });
			}
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function pickCookieFile() {
		try {
			const picked = await open({
				multiple: false,
				directory: false,
				filters: [{ name: "cookies.txt", extensions: ["txt"] }]
			});
			if (typeof picked === "string" && picked) {
				await patch({ cookie_file: picked });
			}
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	function patchRule(id: string, rulePatch: Partial<DownloadRule>) {
		void enqueueUpdate((current) => ({
			download_rules: (current?.download_rules ?? []).map((r) => (r.id === id ? { ...r, ...rulePatch } : r))
		}));
	}

	function addRule() {
		void enqueueUpdate((current) => ({
			download_rules: [
				...(current?.download_rules ?? []),
				{ id: crypto.randomUUID(), pattern: "", dir: "", quality: "", mode: "" }
			]
		}));
	}

	function removeRule(id: string) {
		void enqueueUpdate((current) => ({
			download_rules: (current?.download_rules ?? []).filter((r) => r.id !== id)
		}));
	}

	async function browseRuleDir(id: string) {
		try {
			const picked = await open({ directory: true, multiple: false });
			if (typeof picked === "string" && picked) patchRule(id, { dir: picked });
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function pickCookieBucket() {
		try {
			const picked = await open({
				multiple: false,
				directory: false,
				filters: [{ name: "cookies.txt", extensions: ["txt"] }]
			});
			if (typeof picked !== "string" || !picked) return;
			const stem = picked.split(/[\\/]/).pop()?.replace(/\.txt$/i, "") ?? "bucket";
			const name = window.prompt(t("settings.bucketNamePrompt"), stem);
			if (name === null) return;
			const bucket = {
				id: crypto.randomUUID(),
				name: name.trim() || stem,
				file: picked,
				domain: "",
				added_at: Math.floor(Date.now() / 1000)
			};
			await enqueueUpdate((current) => ({
				cookie_buckets: [...(current?.cookie_buckets ?? []), bucket]
			}));
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function useBucket(bucket: CookieBucket) {
		const ok = await patch({ cookie_file: bucket.file });
		if (ok) toastStore.push(t("settings.cookieBucketUsed", { name: bucket.name }));
	}

	async function renameBucket(bucket: CookieBucket) {
		const name = window.prompt(t("settings.renameBucketPrompt"), bucket.name);
		if (name === null) return;
		const trimmed = name.trim();
		if (!trimmed) return;
		await enqueueUpdate((current) => ({
			cookie_buckets: (current?.cookie_buckets ?? []).map((b) =>
				b.id === bucket.id ? { ...b, name: trimmed } : b
			)
		}));
	}

	async function deleteBucket(bucket: CookieBucket) {
		if (!window.confirm(t("settings.deleteBucketConfirm", { name: bucket.name }))) return;
		await enqueueUpdate((current) => ({
			cookie_buckets: (current?.cookie_buckets ?? []).filter((b) => b.id !== bucket.id),
			cookie_trash: [...(current?.cookie_trash ?? []), bucket],
			...(current?.cookie_file === bucket.file ? { cookie_file: "" } : {})
		}));
	}

	async function restoreBucket(bucket: CookieBucket) {
		await enqueueUpdate((current) => ({
			cookie_trash: (current?.cookie_trash ?? []).filter((b) => b.id !== bucket.id),
			cookie_buckets: [...(current?.cookie_buckets ?? []), bucket]
		}));
	}

	async function deleteForever(bucket: CookieBucket) {
		if (!window.confirm(t("settings.deleteForeverConfirm", { name: bucket.name }))) return;
		await enqueueUpdate((current) => ({
			cookie_trash: (current?.cookie_trash ?? []).filter((b) => b.id !== bucket.id)
		}));
	}

	async function emptyTrash() {
		if (!window.confirm(t("settings.emptyTrashConfirm"))) return;
		await patch({ cookie_trash: [] });
	}

	async function checkBucket(bucket: CookieBucket) {
		try {
			const report = await api.cookieBucketCheck(bucket.file);
			if (report.ok) {
				const parts = [t("settings.cookieCheckSummary", { total: report.total, sites: report.sites.length })];
				if (report.soonest_expiry) {
					parts.push(
						t("settings.cookieSoonestExpiry", {
							time: new Date(report.soonest_expiry * 1000).toLocaleDateString()
						})
					);
				}
				if (report.expired > 0) {
					parts.push(t("settings.cookieExpired", { count: report.expired }));
				}
				toastStore.push(parts.join(" · "));
			} else {
				toastStore.push(
					t("settings.cookieCheckBad", { error: report.error ?? t("settings.cookieInvalid") }),
					"error"
				);
			}
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function installEngine(name: "ytdlp" | "ffmpeg") {
		installBusy = name;
		try {
			await api.engineInstall(name);
			await refreshEngine();
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		} finally {
			installBusy = null;
		}
	}

	function addPreset() {
		void enqueueUpdate((current) => ({
			presets: [
				...(current?.presets ?? []),
				{
					id: crypto.randomUUID(),
					name: t("settings.newPreset"),
					quality: "best",
					mode: "auto",
					dir: "",
					template: "",
					priority: 0,
					speed_limit_mbps: 0
				}
			]
		}));
	}

	function patchPreset(id: string, patch: Partial<import("#lib/api").DownloadPreset>) {
		void enqueueUpdate((current) => ({
			presets: (current?.presets ?? []).map((p) => (p.id === id ? { ...p, ...patch } : p))
		}));
	}

	function removePreset(id: string) {
		void enqueueUpdate((current) => ({
			presets: (current?.presets ?? []).filter((p) => p.id !== id)
		}));
	}

	function addSmartFolder() {
		void enqueueUpdate((current) => ({
			smart_folders: [
				...(current?.smart_folders ?? []),
				{ id: crypto.randomUUID(), name: t("settings.newSmartFolder"), query: "", kind: "all" }
			]
		}));
	}

	function patchSmartFolder(id: string, patch: Partial<import("#lib/api").SmartFolder>) {
		void enqueueUpdate((current) => ({
			smart_folders: (current?.smart_folders ?? []).map((f) => (f.id === id ? { ...f, ...patch } : f))
		}));
	}

	async function pickBackupPath(save: boolean): Promise<string | null> {
		const { save: saveDialog, open: openDialog } = await import("@tauri-apps/plugin-dialog");
		if (save) return await saveDialog({ filters: [{ name: "Getta backup", extensions: ["zip"] }] });
		return await openDialog({ multiple: false, filters: [{ name: "Getta backup", extensions: ["zip"] }] });
	}

	async function doBackup() {
		try {
			const path = await pickBackupPath(true);
			if (!path) return;
			await api.createBackup(path);
			toastStore.push(t("settings.backupDone"), "info");
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function doRestore() {
		try {
			const path = await pickBackupPath(false);
			if (!path) return;
			if (!window.confirm(t("settings.restoreConfirm"))) return;
			const count = await api.restoreBackup(path);
			toastStore.push(t("settings.restoreDone", { count }), "info");
			await settingsStore.load();
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function loadAudit() {
		try {
			auditLines = await api.readAudit();
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	const categories: Array<{ id: Category; label: () => string }> = [
		{ id: "output", label: () => t("settings.catOutput") },
		{ id: "transfer", label: () => t("settings.catTransfer") },
		{ id: "appearance", label: () => t("settings.catAppearance") },
		{ id: "engine", label: () => t("settings.catEngine") },
		{ id: "automation", label: () => t("settings.catAutomation") },
		{ id: "data", label: () => t("settings.catData") }
	];

	const deps = $derived(
		engine
			? ([
					["ytdlp", t("settings.ytdlp"), engine.ytdlp],
					["ffmpeg", t("settings.ffmpeg"), engine.ffmpeg]
				] as Array<["ytdlp" | "ffmpeg", string, EngineStatus["ytdlp"]]>)
			: null
	);
</script>

<div class="page">
	<div class="page-inner">
		<h1 class="page-title">{t("settings.title")}</h1>
		<div class="layout">
			<nav class="cats" aria-label={t("settings.sections")}>
				{#each categories as c (c.id)}
					<button class="cat-item" class:active={category === c.id} aria-current={category === c.id ? "page" : undefined} onclick={() => (category = c.id)}>
						{c.label()}
					</button>
				{/each}
			</nav>

			<div class="panel-col">
				{#if !settings}
					<div class="spinner" role="status"></div>
				{:else if category === "output"}
					<section class="card group">
						<h2 class="section-title">{t("settings.catOutput")}</h2>
						<div class="row">
							<div class="row-text">
								<label class="field-label" for="output-dir">{t("settings.outputDir")}</label>
								<input id="output-dir" class="input" type="text" value={settings.output_dir} onchange={(e) => patch({ output_dir: (e.currentTarget as HTMLInputElement).value })} />
							</div>
							<button class="btn" onclick={pickFolder}>{t("settings.browse")}</button>
						</div>
						<div class="row">
							<div class="row-text">
								<label class="field-label" for="filename-template">{t("settings.filenameTemplate")}</label>
								<input id="filename-template" class="input mono" type="text" value={settings.filename_template} onchange={(e) => patch({ filename_template: (e.currentTarget as HTMLInputElement).value })} />
								<p class="hint">{t("settings.filenameTemplateHint")}</p>
							</div>
						</div>
						{#each [{ id: "organize_by_platform", label: t("settings.organizeByPlatform"), value: settings.organize_by_platform }, { id: "always_ask_path", label: t("settings.alwaysAsk"), value: settings.always_ask_path }, { id: "skip_existing", label: t("settings.skipExisting"), value: settings.skip_existing }, { id: "download_archive", label: t("settings.downloadArchive"), value: settings.download_archive }] as flag (flag.id)}
							<div class="row row-switch">
								<span class="switch-label">{flag.label}</span>
								<button disabled={patching} class="switch" role="switch" aria-checked={flag.value} aria-label={flag.label} onclick={() => patch({ [flag.id]: !flag.value } as Partial<import("#lib/api").AppSettings>)}></button>
							</div>
						{/each}
					<div class="rules-block">
						<div class="rules-head">
							<span class="field-label" style="margin:0">{t("settings.downloadRules")}</span>
							<button class="btn btn-sm" onclick={addRule} disabled={patching}>{t("settings.addRule")}</button>
						</div>
						<p class="hint">{t("settings.downloadRulesHint")}</p>
						{#each settings.download_rules as rule (rule.id)}
							<div class="rule-row">
								<input class="input mono rule-pattern" type="text" placeholder="youtube.com" aria-label={t("settings.rulePattern")} value={rule.pattern} onchange={(e) => patchRule(rule.id, { pattern: (e.currentTarget as HTMLInputElement).value })} />
								<input class="input mono rule-dir" type="text" placeholder="D:\Media" aria-label={t("settings.outputDir")} value={rule.dir} onchange={(e) => patchRule(rule.id, { dir: (e.currentTarget as HTMLInputElement).value })} />
								<input class="input mono rule-quality" type="text" placeholder="best" aria-label={t("settings.ruleQuality")} value={rule.quality} onchange={(e) => patchRule(rule.id, { quality: (e.currentTarget as HTMLInputElement).value })} />
								<select class="input rule-mode" aria-label={t("settings.ruleMode")} value={rule.mode} onchange={(e) => patchRule(rule.id, { mode: (e.currentTarget as HTMLSelectElement).value })}>
									<option value="">{t("settings.ruleModeDefault")}</option>
									<option value="auto">{t("home.modeAuto")}</option>
									<option value="audio">{t("home.modeAudio")}</option>
									<option value="mute">{t("home.modeMute")}</option>
									<option value="subs">{t("home.modeSubs")}</option>
								</select>
								<button disabled={patching} class="btn-icon" title={t("settings.browse")} aria-label={t("settings.browse")} onclick={() => browseRuleDir(rule.id)}><svg viewBox="0 0 24 24"><path d="M4 8V6a2 2 0 0 1 2-2h4l2 2.5h6a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2z" /></svg></button>
								<button disabled={patching} class="btn-icon danger" title={t("settings.removeRule")} aria-label={t("settings.removeRule")} onclick={() => removeRule(rule.id)}><svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6 6 18" /></svg></button>
							</div>
						{/each}
					</div>
					</section>
				{:else if category === "transfer"}
					<section class="card group">
						<h2 class="section-title">{t("settings.catTransfer")}</h2>
						<div class="row">
							<div class="row-text">
								<label class="field-label" for="quality">{t("settings.quality")}</label>
								<select id="quality" class="input" value={settings.video_quality} onchange={(e) => patch({ video_quality: (e.currentTarget as HTMLSelectElement).value })}>
									{#each ["best", "2160", "1440", "1080", "720", "480", "360"] as q (q)}
										<option value={q}>{q === "best" ? t("home.best") : `${q}p`}</option>
									{/each}
								</select>
							</div>
						</div>
						<div class="grid-2">
							<div>
								<label class="field-label" for="concurrency">{t("settings.concurrency")}</label>
								<input id="concurrency" class="input" type="number" min="1" max="10" value={settings.max_concurrent} onchange={(e) => patch({ max_concurrent: Math.max(1, Math.min(10, Number((e.currentTarget as HTMLInputElement).value) || 2)) })} />
							</div>
							<div>
								<label class="field-label" for="fragments">{t("settings.fragments")}</label>
								<input id="fragments" class="input" type="number" min="1" max="32" value={settings.concurrent_fragments} onchange={(e) => patch({ concurrent_fragments: Math.max(1, Math.min(32, Number((e.currentTarget as HTMLInputElement).value) || 8)) })} />
							</div>
							<div>
								<label class="field-label" for="retries">{t("settings.retries")}</label>
								<input id="retries" class="input" type="number" min="0" max="20" value={settings.max_retries} onchange={(e) => patch({ max_retries: Math.max(0, Math.min(20, Number((e.currentTarget as HTMLInputElement).value) || 3)) })} />
							</div>
							<div>
								<label class="field-label" for="speed-limit">{t("settings.speedLimit")}</label>
								<input id="speed-limit" class="input" type="number" min="0" max="1000" value={settings.speed_limit_mbps} onchange={(e) => patch({ speed_limit_mbps: Math.max(0, Number((e.currentTarget as HTMLInputElement).value) || 0) })} />
								<p class="hint">{t("settings.speedLimitHint")}</p>
							</div>
						</div>
						{#each [{ id: "embed_metadata", label: t("settings.embedMetadata"), value: settings.embed_metadata }, { id: "embed_thumbnail", label: t("settings.embedThumbnail"), value: settings.embed_thumbnail }, { id: "write_subtitles", label: t("settings.writeSubs"), value: settings.write_subtitles }, { id: "sponsorblock", label: t("settings.sponsorblock"), value: settings.sponsorblock }, { id: "split_by_chapters", label: t("settings.splitByChapters"), value: settings.split_by_chapters }, { id: "live_from_start", label: t("settings.liveFromStart"), value: settings.live_from_start }, { id: "notifications", label: t("settings.notifications"), value: settings.notifications }, { id: "prevent_sleep", label: t("settings.preventSleep"), value: settings.prevent_sleep }] as flag (flag.id)}
							<div class="row row-switch">
								<span class="switch-label">{flag.label}</span>
								<button disabled={patching} class="switch" role="switch" aria-checked={flag.value} aria-label={flag.label} onclick={() => patch({ [flag.id]: !flag.value } as Partial<import("#lib/api").AppSettings>)}></button>
							</div>
						{/each}
						{#if settings.write_subtitles}
							<div class="row">
								<div class="row-text">
									<label class="field-label" for="sublang">{t("settings.subLang")}</label>
									<input id="sublang" class="input mono" type="text" value={settings.subtitle_lang} onchange={(e) => patch({ subtitle_lang: (e.currentTarget as HTMLInputElement).value })} />
								</div>
							</div>
						{/if}
						<div class="row">
							<div class="row-text">
								<label class="field-label" for="proxy">{t("settings.proxy")}</label>
								<input id="proxy" class="input mono" type="text" placeholder={t("settings.proxyHint")} value={settings.proxy} onchange={(e) => patch({ proxy: (e.currentTarget as HTMLInputElement).value })} />
							</div>
						</div>
					</section>
				{:else if category === "appearance"}
					<section class="card group">
						<h2 class="section-title">{t("settings.catAppearance")}</h2>
						<div class="row">
							<div class="row-text">
								<span class="field-label">{t("settings.theme")}</span>
								<div class="segmented" role="group" aria-label={t("settings.theme")}>
									<button disabled={patching} aria-pressed={settings.theme === "system"} onclick={() => patch({ theme: "system" })}>{t("settings.themeSystem")}</button>
									<button disabled={patching} aria-pressed={settings.theme === "light"} onclick={() => patch({ theme: "light" })}>{t("settings.themeLight")}</button>
									<button disabled={patching} aria-pressed={settings.theme === "dark"} onclick={() => patch({ theme: "dark" })}>{t("settings.themeDark")}</button>
								</div>
							</div>
						</div>
						<div class="row">
							<div class="row-text">
								<label class="field-label" for="language">{t("settings.language")}</label>
									<select id="language" class="input" value={settings.language} onchange={(e) => patch({ language: (e.currentTarget as HTMLSelectElement).value as "en" | "id" })}>
									<option value="en">English</option>
									<option value="id">Bahasa Indonesia</option>
								</select>
							</div>
						</div>
						<div class="row row-switch">
							<span class="switch-label">{t("settings.autostart")}</span>
							<button class="switch" role="switch" aria-checked={autostartEnabled} aria-label={t("settings.autostart")} onclick={toggleAutostart}></button>
						</div>
						<div class="row row-switch">
							<span class="switch-label">{t("settings.closeToTray")}</span>
							<button disabled={patching} class="switch" role="switch" aria-checked={settings.close_to_tray} aria-label={t("settings.closeToTray")} onclick={() => patch({ close_to_tray: !settings.close_to_tray })}></button>
						</div>
						<div class="row">
							<span class="switch-label">{t("settings.reduceMotion")}</span>
							<button disabled={patching} class="switch" role="switch" aria-checked={settings.reduce_motion} aria-label={t("settings.reduceMotion")} onclick={() => patch({ reduce_motion: !settings.reduce_motion })}></button>
						</div>
						<div class="row">
							<label class="field-label" for="accent-color">{t("settings.accent")}<input id="accent-color" class="input" type="color" value={settings.accent || "#00C896"} onchange={(e) => patch({ accent: e.currentTarget.value })} /></label>
						</div>
						<div class="row">
							<span class="switch-label">{t("settings.compactMode")}</span>
							<button disabled={patching} class="switch" role="switch" aria-checked={settings.compact_mode} aria-label={t("settings.compactMode")} onclick={() => patch({ compact_mode: !settings.compact_mode })}></button>
						</div>
					</section>
				{:else if category === "engine"}
					<section class="card group">
						<h2 class="section-title">{t("settings.catEngine")}</h2>
						{#if deps}
							{#each deps as [name, label, info] (name)}
								<div class="row dep-row">
									<div class="row-text">
										<span class="dep-name">{label}</span>
										<span class="muted dep-status">
											{#if info.found}
												{t("settings.installed")}{info.version ? ` · ${info.version}` : ""}{info.source ? ` · ${info.source}` : ""}
											{:else}
												{t("settings.notInstalled")}
											{/if}
										</span>
									</div>
									<button class="btn btn-sm" disabled={installBusy !== null} onclick={() => installEngine(name as "ytdlp" | "ffmpeg")}>
										{#if installBusy === name}{t("common.installing")}{:else if info.found}{t("settings.updateNow")}{:else}{t("settings.install")}{/if}
									</button>
								</div>
							{/each}
						{:else}
							<div class="spinner" role="status"></div>
						{/if}
						<div class="row">
							<div class="row-text">
								<label class="field-label" for="cookies">{t("settings.cookiesFromBrowser")}</label>
								<input id="cookies" class="input mono" type="text" placeholder="chrome, firefox, edge, brave, safari" value={settings.cookies_from_browser} onchange={(e) => patch({ cookies_from_browser: (e.currentTarget as HTMLInputElement).value })} />
								<p class="hint">{t("settings.cookiesHint")}</p>
								<label class="field-label" for="cookie-file" style="margin-top:12px">{t("settings.cookieFile")}</label>
								<div class="cookie-file-row">
									<code class="cookie-file-value">{settings.cookie_file || "-"}</code>
									<button class="btn btn-sm" onclick={pickCookieFile}>{t("settings.cookieFilePick")}</button>
									{#if settings.cookie_file}
										<button disabled={patching} class="btn btn-sm btn-ghost" onclick={() => patch({ cookie_file: "" })}>{t("settings.cookieFileClear")}</button>
									{/if}
								</div>
								<p class="hint">{t("settings.cookieFileWarning")}</p>
                                <div class="bucket-block">
                                    <div class="bucket-head">
                                        <span class="field-label" style="margin:0">{t("settings.cookieBuckets")}</span>
                                        <button class="btn btn-sm" disabled={patching} onclick={pickCookieBucket}>{t("settings.addBucket")}</button>
                                    </div>
                                    <p class="hint">{t("settings.cookieBucketsHint")}</p>
                                    {#each settings.cookie_buckets as bucket (bucket.id)}
                                        <div class="bucket-row" class:active={settings.cookie_file === bucket.file}>
                                            <div class="bucket-row-text">
                                                <span class="bucket-name">{bucket.name}{#if settings.cookie_file === bucket.file} <span class="bucket-active-chip mono">{t("settings.cookieActive")}</span>{/if}</span>
                                                <span class="bucket-file mono" title={bucket.file}>{bucket.file}</span>
                                            </div>
                                            <div class="bucket-actions">
                                                <button disabled={patching} class="btn btn-sm" onclick={() => useBucket(bucket)}>{t("settings.cookieUse")}</button>
                                                <button class="btn-icon" title={t("settings.cookieCheck")} aria-label={t("settings.cookieCheck")} onclick={() => checkBucket(bucket)}><svg viewBox="0 0 24 24"><path d="M20 6 9 17l-5-5" /></svg></button>
                                                <button disabled={patching} class="btn-icon" title={t("settings.renameBucket")} aria-label={t("settings.renameBucket")} onclick={() => renameBucket(bucket)}><svg viewBox="0 0 24 24"><path d="M4 20h4L19 9a2.1 2.1 0 0 0-3-3L5 17z" /></svg></button>
                                                <button disabled={patching} class="btn-icon danger" title={t("settings.deleteBucket")} aria-label={t("settings.deleteBucket")} onclick={() => deleteBucket(bucket)}><svg viewBox="0 0 24 24"><path d="M5 7h14M10 11v6M14 11v6M8 7l1-2h6l1 2" /></svg></button>
                                            </div>
                                        </div>
                                    {:else}
                                        <p class="hint">{t("settings.cookieBucketsEmpty")}</p>
                                    {/each}
                                    {#if settings.cookie_trash.length > 0}
                                        <div class="trash-block">
                                            <div class="trash-head">
                                                <span class="field-label" style="margin:0">{t("settings.cookieTrash", { count: settings.cookie_trash.length })}</span>
                                                <button class="btn btn-sm btn-ghost" onclick={emptyTrash}>{t("settings.clearTrash")}</button>
                                            </div>
                                            {#each settings.cookie_trash as bucket (bucket.id)}
                                                <div class="trash-row">
                                                    <span class="trash-name mono" title={bucket.file}>{bucket.name}</span>
                                                    <div class="trash-actions">
                                                        <button disabled={patching} class="btn btn-sm btn-ghost" onclick={() => restoreBucket(bucket)}>{t("settings.restoreBucket")}</button>
                                                        <button disabled={patching} class="btn-icon danger" title={t("settings.deleteForever")} aria-label={t("settings.deleteForever")} onclick={() => deleteForever(bucket)}><svg viewBox="0 0 24 24"><path d="M5 7h14M10 11v6M14 11v6M8 7l1-2h6l1 2" /></svg></button>
                                                    </div>
                                                </div>
                                            {/each}
                                        </div>
                                    {/if}
                                </div>
							</div>
						</div>
					</section>
				{:else if category === "automation"}
					<section class="card group">
						<h2 class="section-title">{t("settings.presets")}</h2>
						<div class="row">
							<button class="btn btn-sm" onclick={addPreset} disabled={patching}>{t("settings.addPreset")}</button>
						</div>
						{#each settings.presets as preset (preset.id)}
							<div class="preset-row">
								<input class="input" value={preset.name} aria-label={t("settings.presetName")} onchange={(e) => patchPreset(preset.id, { name: e.currentTarget.value })} />
								<input class="input" value={preset.quality} aria-label={t("settings.videoQuality")} onchange={(e) => patchPreset(preset.id, { quality: e.currentTarget.value })} />
								<select class="input" value={preset.mode} aria-label={t("settings.presetMode")} onchange={(e) => patchPreset(preset.id, { mode: e.currentTarget.value })}>
									<option value="auto">{t("home.modeAuto")}</option>
									<option value="audio">{t("home.modeAudio")}</option>
									<option value="mute">{t("home.modeMute")}</option>
									<option value="subs">{t("home.modeSubs")}</option>
								</select>
								<button class="btn-icon danger" title={t("settings.removeRule")} onclick={() => removePreset(preset.id)}><svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6 6 18" /></svg></button>
							</div>
						{/each}
					</section>

					<section class="card group">
						<h2 class="section-title">{t("settings.smartFolders")}</h2>
						<div class="row">
							<button class="btn btn-sm" onclick={addSmartFolder} disabled={patching}>{t("settings.addSmartFolder")}</button>
						</div>
						{#each settings.smart_folders as folder (folder.id)}
							<div class="preset-row">
								<input class="input" value={folder.name} aria-label={t("settings.smartName")} onchange={(e) => patchSmartFolder(folder.id, { name: e.currentTarget.value })} />
								<input class="input" value={folder.query} aria-label={t("settings.smartQuery")} onchange={(e) => patchSmartFolder(folder.id, { query: e.currentTarget.value })} />
							</div>
						{/each}
					</section>

					<section class="card group">
						<h2 class="section-title">{t("settings.retryPolicy")}</h2>
						<div class="row">
							<label class="field-label">{t("settings.retryDelay")}<input class="input" type="number" min="1" max="600" value={settings.retry_delay_secs} onchange={(e) => patch({ retry_delay_secs: Number(e.currentTarget.value) })} /></label>
						</div>
						<div class="row">
							<label class="field-label">{t("settings.retryMultiplier")}<input class="input" type="number" min="1" max="10" step="0.1" value={settings.retry_multiplier} onchange={(e) => patch({ retry_multiplier: Number(e.currentTarget.value) })} /></label>
						</div>
						<div class="row">
							<button disabled={patching} class="switch" role="switch" aria-checked={settings.quiet_hours_enabled} aria-label={t("settings.quietHours")} onclick={() => patch({ quiet_hours_enabled: !settings.quiet_hours_enabled })}></button>
							<span>{t("settings.quietHours")}</span>
						</div>
						<div class="row">
							<input class="input" type="number" min="0" max="23" value={settings.quiet_start_hour} aria-label={t("settings.quietStart")} onchange={(e) => patch({ quiet_start_hour: Number(e.currentTarget.value) })} />
							<input class="input" type="number" min="0" max="23" value={settings.quiet_end_hour} aria-label={t("settings.quietEnd")} onchange={(e) => patch({ quiet_end_hour: Number(e.currentTarget.value) })} />
						</div>
						<div class="row">
							<label class="field-label">{t("settings.postAction")}<select class="input" value={settings.post_action} onchange={(e) => patch({ post_action: e.currentTarget.value })}>
								<option value="none">{t("settings.postNone")}</option>
								<option value="move">{t("settings.postMove")}</option>
								<option value="copy">{t("settings.postCopy")}</option>
							</select></label>
							<input class="input" value={settings.post_action_dir} placeholder={t("settings.outputDir")} aria-label={t("settings.outputDir")} onchange={(e) => patch({ post_action_dir: e.currentTarget.value })} />
						</div>
					</section>

					<section class="card group">
						<h2 class="section-title">{t("settings.webhook")}</h2>
						<div class="row">
							<input class="input" value={settings.webhook_url} placeholder="http://127.0.0.1:8080/hook" aria-label={t("settings.webhook")} onchange={(e) => patch({ webhook_url: e.currentTarget.value })} />
						</div>
						<div class="row">
							<input class="input" value={settings.webhook_events} aria-label={t("settings.webhookEvents")} onchange={(e) => patch({ webhook_events: e.currentTarget.value })} />
						</div>
						<div class="row">
							<button disabled={patching} class="switch" role="switch" aria-checked={settings.clipboard_monitor} aria-label={t("settings.clipboardMonitor")} onclick={() => patch({ clipboard_monitor: !settings.clipboard_monitor })}></button>
							<span>{t("settings.clipboardMonitor")}</span>
						</div>
					</section>
				{:else if category === "data"}
					<section class="card group">
						<h2 class="section-title">{t("settings.backup")}</h2>
						<div class="row">
							<button class="btn btn-sm" onclick={doBackup}>{t("settings.createBackup")}</button>
							<button class="btn btn-sm" onclick={doRestore}>{t("settings.restoreBackup")}</button>
						</div>
						<p class="muted">{t("settings.backupHint")}</p>
					</section>
					<section class="card group">
						<h2 class="section-title">{t("settings.audit")}</h2>
						<div class="row"><button class="btn btn-sm" onclick={loadAudit}>{t("settings.loadAudit")}</button></div>
						{#if auditLines.length > 0}
							<ul class="audit-list">
								{#each auditLines as line (line)}
									<li class="mono">{line}</li>
								{/each}
							</ul>
						{/if}
					</section>
				{/if}
			</div>
		</div>
	</div>
</div>

<style>
	.preset-row,
	.audit-list {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		margin-top: 8px;
	}

	.audit-list {
		flex-direction: column;
		max-height: 260px;
		overflow: auto;
	}

	.page-title {
		font-family: var(--font-display);
		font-size: 1.9rem;
		font-weight: 700;
		margin-bottom: 16px;
	}

	.panel-col {
		min-width: 0;
	}

	.layout {
		display: grid;
		grid-template-columns: 200px 1fr;
		gap: 18px;
		align-items: start;
	}

	.cats {
		display: flex;
		flex-direction: column;
		gap: 4px;
		position: sticky;
		top: 16px;
	}

	.cat-item {
		text-align: left;
		border: none;
		background: transparent;
		padding: 10px 12px;
		border-radius: var(--r-md);
		font-size: 14px;
		font-weight: 600;
		color: var(--muted);
		cursor: pointer;
	}

	.cat-item:hover {
		background: var(--card-2);
		color: var(--text);
	}

	.cat-item.active {
		background: var(--accent-soft);
		color: var(--accent);
	}

	.group {
		padding: 20px;
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.row {
		display: flex;
		align-items: center;
		gap: 14px;
		justify-content: space-between;
	}

	.row-text {
		flex: 1;
		min-width: 0;
	}

	.row-switch {
		justify-content: space-between;
	}

	.switch-label {
		font-size: 14px;
		font-weight: 550;
	}

	.rules-block {
		display: flex;
		flex-direction: column;
		gap: 8px;
		border-top: 1px solid var(--border);
		padding-top: 14px;
		margin-top: 14px;
	}

	.rules-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
	}

	.rule-row {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.rule-pattern {
		flex: 1;
		min-width: 0;
	}

	.rule-dir,
	.rule-quality,
	.rule-mode {
		flex: 1.2;
		min-width: 0;
	}

	.grid-2 {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 12px;
	}

	.dep-row {
		padding: 10px 12px;
		background: var(--card-2);
		border-radius: var(--r-md);
	}

	.cookie-file-row {
		display: flex;
		align-items: center;
		gap: 8px;
		margin: 6px 0;
	}

	.cookie-file-value {
		flex: 1;
		min-width: 0;
		font-family: var(--font-mono);
		font-size: 10.5px;
		border: 1px solid var(--border);
		padding: 7px 9px;
		border-radius: var(--r-sm);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		color: var(--muted);
	}

	.bucket-block {
		display: flex;
		flex-direction: column;
		gap: 8px;
		border-top: 1px solid var(--border);
		padding-top: 14px;
		margin-top: 10px;
	}

	.bucket-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
	}

	.bucket-row {
		display: flex;
		align-items: center;
		gap: 10px;
		justify-content: space-between;
		border: 1px solid var(--border);
		border-radius: var(--r-md);
		padding: 8px 10px;
	}

	.bucket-row.active {
		border-color: var(--accent);
	}

	.bucket-row-text {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
		flex: 1;
	}

	.bucket-name {
		font-size: 12px;
		font-weight: 650;
	}

	.bucket-active-chip {
		font-size: 9px;
		color: var(--accent);
	}

	.bucket-file {
		font-size: 9.5px;
		color: var(--muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.bucket-actions {
		display: flex;
		align-items: center;
		gap: 6px;
		flex: none;
	}

	.trash-block {
		display: flex;
		flex-direction: column;
		gap: 6px;
		border: 1px dashed var(--border);
		border-radius: var(--r-md);
		padding: 10px;
		margin-top: 6px;
	}

	.trash-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
	}

	.trash-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
	}

	.trash-name {
		font-size: 11px;
		color: var(--muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.trash-actions {
		display: flex;
		align-items: center;
		gap: 6px;
		flex: none;
	}

	.dep-name {
		font-weight: 650;
		display: block;
	}

	.dep-status {
		font-size: 12px;
	}

	@media (max-width: 860px) {
		.layout {
			grid-template-columns: 1fr;
		}

		.cats {
			position: static;
			flex-direction: row;
			overflow-x: auto;
			padding-bottom: 4px;
		}

		.cat-item {
			white-space: nowrap;
		}

		.grid-2 {
			grid-template-columns: 1fr;
		}
	}
</style>
