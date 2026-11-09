/** Turn raw engine stderr into something actionable. */
export function humanize(message: string | null | undefined): string {
	const text = (message ?? "").trim();
	if (!text) return "Unknown error.";
	if (/failed to decrypt with dpapi/i.test(text)) {
		return "Browser cookies can't be read on Chrome 127+ (DPAPI blocked). Use Firefox for cookies, or export cookies.txt and pick it in Settings, Engine, Cookie file.";
	}
	if (/this browser|could not decrypt/i.test(text) && /firefox|chrome|edge|brave|safari/i.test(text)) {
		return "Browser cookies can't be read. Use Firefox for cookies, or export cookies.txt (free for any browser) and pick it in Settings, Engine, Cookie file.";
	}
	if (/sign in|login|authentication/i.test(text) && /cookie|session/i.test(text)) {
		return "This site needs a logged-in session. Set your browser or a cookies.txt file in Settings, Engine.";
	}
	if (/rate.?limit|429|too many requests/i.test(text)) {
		return "Site rate limited this link. Try again in a minute.";
	}
	return text;
}
