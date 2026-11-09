<script lang="ts">
	import { onMount } from "svelte";
	import { open } from "@tauri-apps/plugin-dialog";
	import { isEnabled, enable, disable } from "@tauri-apps/plugin-autostart";
	import { api, type EngineStatus } from "#lib/api";
	import { settingsStore } from "#lib/stores/settings.svelte";
	import { toastStore } from "#lib/stores/toast.svelte";
	import { t } from "#lib/i18n.svelte";

	type Category = "output" | "transfer" | "appearance" | "engine";
	let category = $state<Category>("output");
	let engine = $state<EngineStatus | null>(null);
	let installBusy = $state<string | null>(null);
	let autostartEnabled = $state(false);

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
			autostartEnabled = next;
			await settingsStore.update({ autostart: next });
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

	async function patch(p: Partial<import("#lib/api").AppSettings>) {
		try {
			await settingsStore.update(p);
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
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

	const categories: Array<{ id: Category; label: () => string }> = [
		{ id: "output", label: () => t("settings.catOutput") },
		{ id: "transfer", label: () => t("settings.catTransfer") },
		{ id: "appearance", label: () => t("settings.catAppearance") },
		{ id: "engine", label: () => t("settings.catEngine") }
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
			<nav class="cats" aria-label="Settings sections">
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
						{#each [{ id: "organize_by_platform", label: t("settings.organizeByPlatform"), value: settings.organize_by_platform }, { id: "always_ask_path", label: t("settings.alwaysAsk"), value: settings.always_ask_path }, { id: "skip_existing", label: t("settings.skipExisting"), value: settings.skip_existing }] as flag (flag.id)}
							<div class="row row-switch">
								<span class="switch-label">{flag.label}</span>
								<button class="switch" role="switch" aria-checked={flag.value} aria-label={flag.label} onclick={() => patch({ [flag.id]: !flag.value } as Partial<import("#lib/api").AppSettings>)}></button>
							</div>
						{/each}
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
								<button class="switch" role="switch" aria-checked={flag.value} aria-label={flag.label} onclick={() => patch({ [flag.id]: !flag.value } as Partial<import("#lib/api").AppSettings>)}></button>
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
									<button aria-pressed={settings.theme === "system"} onclick={() => patch({ theme: "system" })}>{t("settings.themeSystem")}</button>
									<button aria-pressed={settings.theme === "light"} onclick={() => patch({ theme: "light" })}>{t("settings.themeLight")}</button>
									<button aria-pressed={settings.theme === "dark"} onclick={() => patch({ theme: "dark" })}>{t("settings.themeDark")}</button>
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
							<button class="switch" role="switch" aria-checked={settings.close_to_tray} aria-label={t("settings.closeToTray")} onclick={() => patch({ close_to_tray: !settings.close_to_tray })}></button>
						</div>
						<div class="row row-switch">
							<span class="switch-label">{t("settings.reduceMotion")}</span>
							<button class="switch" role="switch" aria-checked={settings.reduce_motion} aria-label={t("settings.reduceMotion")} onclick={() => patch({ reduce_motion: !settings.reduce_motion })}></button>
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
									<code class="cookie-file-value">{settings.cookie_file || "—"}</code>
									<button class="btn btn-sm" onclick={pickCookieFile}>{t("settings.cookieFilePick")}</button>
									{#if settings.cookie_file}
										<button class="btn btn-sm btn-ghost" onclick={() => patch({ cookie_file: "" })}>{t("settings.cookieFileClear")}</button>
									{/if}
								</div>
								<p class="hint">{t("settings.cookieFileWarning")}</p>
							</div>
						</div>
					</section>
				{/if}
			</div>
		</div>
	</div>
</div>

<style>
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
