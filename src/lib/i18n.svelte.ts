import en from "./i18n/en.json";
import id from "./i18n/id.json";

export type Locale = "en" | "id";

const dictionaries: Record<Locale, Record<string, unknown>> = { en, id };

/** Reactive locale container. Read `i18n.locale` in templates to stay in sync. */
export const i18n = $state({ locale: "en" as Locale });

export function setLocale(next: Locale | string | null | undefined): void {
	i18n.locale = next === "id" ? "id" : "en";
}

function lookup(dict: Record<string, unknown>, path: string[]): unknown {
	let node: unknown = dict;
	for (const part of path) {
		if (node == null || typeof node !== "object") return undefined;
		node = (node as Record<string, unknown>)[part];
	}
	return node;
}

export function t(key: string, vars?: Record<string, string | number>): string {
	const path = key.split(".");
	let value = lookup(dictionaries[i18n.locale], path);
	if (typeof value !== "string") value = lookup(dictionaries.en, path);
	let text = typeof value === "string" ? value : key;
	if (vars) {
		for (const [name, replacement] of Object.entries(vars)) {
			text = text.replaceAll(`{${name}}`, String(replacement));
		}
	}
	return text;
}
