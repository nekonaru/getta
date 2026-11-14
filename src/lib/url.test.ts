import { describe, expect, it } from "vitest";
import { extractLink, hasUsableLink } from "./url";

describe("extractLink", () => {
	it("reads a bare YouTube URL from the clipboard", () => {
		expect(extractLink("https://www.youtube.com/watch?v=dQw4w9WgXcQ")).toEqual({
			url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
			guessedScheme: false
		});
	});

	it("finds a link inside a sentence", () => {
		expect(extractLink("watch this https://youtu.be/dQw4w9WgXcQ it is great")).toEqual({
			url: "https://youtu.be/dQw4w9WgXcQ",
			guessedScheme: false
		});
	});

	it("unwraps Markdown-style links", () => {
		expect(extractLink("[that video](https://www.youtube.com/watch?v=abc123)")).toEqual({
			url: "https://www.youtube.com/watch?v=abc123",
			guessedScheme: false
		});
	});

	it("strips trailing punctuation from prose", () => {
		expect(extractLink("cek link ini: https://www.youtube.com/watch?v=abc123.")).toEqual({
			url: "https://www.youtube.com/watch?v=abc123",
			guessedScheme: false
		});
	});

	it("handles invisible marks around the link", () => {
		expect(extractLink("﻿https://www.youtube.com/watch?v=abc123​")).toEqual({
			url: "https://www.youtube.com/watch?v=abc123",
			guessedScheme: false
		});
	});

	it("guesses https for schemeless hosts", () => {
		expect(extractLink("youtube.com/watch?v=abc123")).toEqual({
			url: "https://youtube.com/watch?v=abc123",
			guessedScheme: true
		});
	});

	it("guesses https for youtu.be share links", () => {
		expect(extractLink("youtu.be/dQw4w9WgXcQ")).toEqual({
			url: "https://youtu.be/dQw4w9WgXcQ",
			guessedScheme: true
		});
	});

	it("accepts magnet links", () => {
		expect(extractLink("magnet:?xt=urn:btih:abcdef123456")?.url).toBe("magnet:?xt=urn:btih:abcdef123456");
	});

	it("rejects when there is no link at all", () => {
		expect(extractLink("not a link at all")).toBeNull();
		expect(extractLink("")).toBeNull();
		expect(extractLink(null)).toBeNull();
		expect(extractLink(undefined)).toBeNull();
		expect(hasUsableLink("just words")).toBe(false);
	});

	it("ignores the unsupported getta and http schemes for fetching", () => {
		// getta:// is a deep link, not a fetchable URL.
		expect(extractLink("getta://https://www.youtube.com/watch?v=x")?.url).toBe(
			"https://www.youtube.com/watch?v=x"
		);
	});
});
