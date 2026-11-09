<script lang="ts">
	import { onMount } from "svelte";
	import { open } from "@tauri-apps/plugin-dialog";
	import { api, type EngineStatus } from "#lib/api";
	import { settingsStore } from "#lib/stores/settings.svelte";
	import { toastStore } from "#lib/stores/toast.svelte";
	import { t } from "#lib/i18n.svelte";

	const settings = $derived(settingsStore.value);
	const show = $derived(settings != null && !settings.onboarding_completed);

	let step = $state(1);
	let engine = $state<EngineStatus | null>(null);
	let installBusy = $state(false);
	let folder = $state<string>("");

	onMount(() => {
		void refreshEngine();
		folder = settingsStore.value?.output_dir ?? "";
	});

	async function refreshEngine() {
		try {
			engine = await api.engineStatus();
		} catch {
			engine = null;
		}
	}

	const engineMissing = $derived(engine ? !engine.ytdlp.found || !engine.ffmpeg.found : false);

	const engineItems = $derived(
		engine
			? ([
					["ytdlp", t("settings.ytdlp"), engine.ytdlp.found],
					["ffmpeg", t("settings.ffmpeg"), engine.ffmpeg.found]
				] as Array<[string, string, boolean]>)
			: []
	);

	async function installEverything() {
		installBusy = true;
		try {
			const jobs: Array<"ytdlp" | "ffmpeg"> = [];
			if (engine && !engine.ytdlp.found) jobs.push("ytdlp");
			if (engine && !engine.ffmpeg.found) jobs.push("ffmpeg");
			for (const job of jobs) {
				try {
					await api.engineInstall(job);
				} catch (e) {
					toastStore.push(e instanceof Error ? e.message : String(e), "error");
				}
			}
			await refreshEngine();
		} finally {
			installBusy = false;
		}
	}

	async function pickFolder() {
		try {
			const picked = await open({ directory: true, multiple: false });
			if (typeof picked === "string" && picked) {
				folder = picked;
				await settingsStore.update({ output_dir: picked });
			}
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function finish() {
		try {
			await settingsStore.update({ onboarding_completed: true });
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function skip() {
		try {
			await settingsStore.update({ onboarding_completed: true });
		} catch {
			/* store noise */
		}
	}
</script>

{#if show}
	<div class="veil" role="dialog" aria-modal="true" aria-label="Getta setup">
		<div class="modal onboarding">
			<div class="dots" aria-hidden="true">
				{#each [1, 2, 3] as dot (dot)}
					<span class="dot" class:active={step === dot} class:done={step > dot}></span>
				{/each}
			</div>

			{#if step === 1}
				<h2 class="ob-title">{t("onboarding.step1Title")}</h2>
				<p class="ob-body">{t("onboarding.step1Body")}</p>

				<div class="ob-field">
					<label class="field-label" for="ob-lang">{t("onboarding.language")}</label>
					<select id="ob-lang" class="input" value={settings?.language ?? "en"} onchange={(e) => settingsStore.update({ language: (e.currentTarget as HTMLSelectElement).value as "en" | "id" })}>
						<option value="en">English</option>
						<option value="id">Bahasa Indonesia</option>
					</select>
				</div>

				<div class="ob-field">
					<span class="field-label">{t("onboarding.theme")}</span>
					<div class="segmented" role="group" aria-label={t("onboarding.theme")}>
						<button aria-pressed={(settings?.theme ?? "system") === "system"} onclick={() => settingsStore.update({ theme: "system" })}>{t("onboarding.themeSystem")}</button>
						<button aria-pressed={settings?.theme === "light"} onclick={() => settingsStore.update({ theme: "light" })}>{t("onboarding.themeLight")}</button>
						<button aria-pressed={settings?.theme === "dark"} onclick={() => settingsStore.update({ theme: "dark" })}>{t("onboarding.themeDark")}</button>
					</div>
				</div>

				<div class="ob-foot">
					<button class="btn btn-ghost btn-sm" onclick={skip}>{t("onboarding.skip")}</button>
					<button class="btn btn-accent" onclick={() => (step = 2)}>{t("onboarding.next")}</button>
				</div>
			{:else if step === 2}
				<h2 class="ob-title">{t("onboarding.step2Title")}</h2>
				<p class="ob-body">{t("onboarding.step2Body")}</p>

				{#if engine}
					<ul class="engine-list">
						{#each engineItems as [name, label, found] (name)}
							<li class="engine-item">
								<span class="engine-name mono">{label}</span>
								<span class="engine-state" class:ok={found}>{found ? t("settings.installed") : t("settings.notInstalled")}</span>
							</li>
						{/each}
					</ul>
				{:else}
					<div class="spinner" role="status"></div>
				{/if}

				<div class="ob-foot">
					<button class="btn btn-ghost btn-sm" onclick={() => (step = 1)}>{t("onboarding.back")}</button>
					{#if engineMissing}
						<button class="btn btn-accent" disabled={installBusy} onclick={installEverything}>
							{#if installBusy}<span class="spinner"></span>{t("onboarding.installing")}{:else}{t("onboarding.installEngine")}{/if}
						</button>
					{:else}
						<button class="btn btn-accent" onclick={() => (step = 3)}>{t("onboarding.next")}</button>
					{/if}
				</div>
			{:else}
				<h2 class="ob-title">{t("onboarding.step3Title")}</h2>
				<p class="ob-body">{t("onboarding.step3Body")}</p>

				<div class="ob-field">
					<span class="field-label">{t("settings.outputDir")}</span>
					<div class="folder-row">
						<code class="folder-value">{folder}</code>
						<button class="btn btn-sm" onclick={pickFolder}>{t("onboarding.chooseFolder")}</button>
					</div>
				</div>

				<div class="ob-foot">
					<button class="btn btn-ghost btn-sm" onclick={() => (step = 2)}>{t("onboarding.back")}</button>
					<button class="btn btn-accent" onclick={finish}>{t("onboarding.finish")}</button>
				</div>
			{/if}
		</div>
	</div>
{/if}

<style>
	.onboarding {
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.dots {
		display: flex;
		gap: 6px;
	}

	.dot {
		width: 22px;
		height: 4px;
		border-radius: var(--r-pill);
		background: var(--card-2);
		transition: background 0.2s ease;
	}

	.dot.active {
		background: var(--accent);
	}

	.dot.done {
		background: var(--accent);
		opacity: 0.45;
	}

	.ob-title {
		font-family: var(--font-display);
		font-weight: 800;
		font-size: 1.5rem;
		letter-spacing: -0.02em;
		line-height: 1.15;
	}

	.ob-body {
		color: var(--muted);
		margin: 0;
		font-size: 13px;
	}

	.ob-field {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.ob-foot {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		margin-top: 4px;
	}

	.engine-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.engine-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		background: var(--card-2);
		padding: 10px 12px;
		border-radius: var(--r-md);
	}

	.engine-name {
		font-size: 13px;
	}

	.engine-state {
		font-size: 12px;
		font-weight: 650;
		color: var(--warn);
	}

	.engine-state.ok {
		color: var(--good);
	}

	.folder-row {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.folder-value {
		flex: 1;
		font-family: var(--font-mono);
		font-size: 11px;
		border: 1px solid var(--border);
		border-radius: var(--r-sm);
		padding: 8px 10px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
</style>
