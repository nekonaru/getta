/**
 * Link extraction from arbitrary clipboard text.
 *
 * Real-world clipboard contents are messy: Markdown links, quotes,
 * trailing punctuation, schemeless hosts, invisible marks. This module
 * finds the first usable link without depending on where it starts.
 */

// URL charset: no whitespace, quotes, angle brackets, backticks or square
// brackets. Parentheses are allowed so Wikipedia-style URLs survive.
const URL_CHARS = String.raw`[^\s"'<>` + "`" + String.raw`\]]`;
const SCHEME_LINK = new RegExp(`https?:\\/\\/${URL_CHARS}+|magnet:\\?${URL_CHARS}+|p2p:${URL_CHARS}+`, "i");
const HOST_LIKE = new RegExp(
	`(?:[a-z0-9-]+\\.)+(?:com|net|org|id|io|tv|co|me|gg|ly|to|cc|app|dev|live|music|be)(?:\\/${URL_CHARS}*)?`,
	"i"
);

/** Strip trailing characters that belong to the prose, not the URL. */
function trimTrailing(url: string): string {
	let end = url.length;
	while (end > 0) {
		const last = url[end - 1];
		const opens = (url.match(/\(/g) ?? []).length;
		const closes = (url.match(/\)/g) ?? []).length;
		if (last === ")" && closes > opens) {
			end -= 1;
			continue;
		}
		if (/[.,;:!?'"}\]"„””']/.test(last) && last !== "?") {
			end -= 1;
			continue;
		}
		// A bare trailing "?" means an empty query, drop it too.
		if (last === "?") {
			end -= 1;
			continue;
		}
		break;
	}
	return url.slice(0, end);
}

export interface ExtractedLink {
	/** The link, ready to hand to the engine (always has a scheme). */
	url: string;
	/** True when a host was guessed and https:// was prepended. */
	guessedScheme: boolean;
}

export function extractLink(text: string | null | undefined): ExtractedLink | null {
	const cleaned = (text ?? "").replace(/[​‌‍﻿⁠]/g, "");
	if (!cleaned.trim()) return null;

	const schemeMatch = cleaned.match(SCHEME_LINK);
	if (schemeMatch) {
		const candidate = trimTrailing(schemeMatch[0]);
		try {
			new URL(candidate);
			return { url: candidate, guessedScheme: false };
		} catch {
			/* not a valid URL after all; try bare-host matching */
		}
	}

	const hostMatch = cleaned.match(HOST_LIKE);
	if (hostMatch) {
		const candidate = `https://${trimTrailing(hostMatch[0])}`;
		try {
			new URL(candidate);
			return { url: candidate, guessedScheme: true };
		} catch {
			return null;
		}
	}

	return null;
}

/** True when the text contains at least one link the app can fetch. */
export function hasUsableLink(text: string | null | undefined): boolean {
	return extractLink(text) !== null;
}
