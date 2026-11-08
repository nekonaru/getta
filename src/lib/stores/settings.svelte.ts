import { api, type AppSettings } from "$lib/api";
import { setLocale } from "$lib/i18n";

export type ThemeSetting = "system" | "light" | "dark";

class SettingsStore {
	value = $state<AppSettings | null>(null);
	loading = $state(false);
	error = $state<string | null>(null);

	async load(): Promise<void> {
		if (this.loading) return;
		this.loading = true;
		try {
			const settings = await api.settingsGet();
			this.value = settings;
			this.error = null;
			applyAppearance(settings);
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
		} finally {
			this.loading = false;
		}
	}

	async update(patch: Partial<AppSettings>): Promise<void> {
		try {
			const settings = await api.settingsUpdate(patch);
			this.value = settings;
			this.error = null;
			applyAppearance(settings);
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
			throw e;
		}
	}

	async reset(): Promise<void> {
		const settings = await api.settingsReset();
		this.value = settings;
		applyAppearance(settings);
	}
}

function applyAppearance(settings: AppSettings): void {
	try {
		localStorage.setItem("getta.theme", settings.theme);
	} catch {
		/* storage unavailable: theme still applies for this session */
	}
	document.documentElement.dataset.theme = settings.theme;
	document.documentElement.dataset.reduceMotion = settings.reduce_motion ? "true" : "false";
	setLocale(settings.language);
}

export const settingsStore = new SettingsStore();
