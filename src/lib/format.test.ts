import { describe, expect, it } from "vitest";
import { formatBytes, formatDuration, formatEta, formatSpeed } from "./format";

describe("formatBytes", () => {
	it("formats sub-KB bytes", () => {
		expect(formatBytes(512)).toBe("512 B");
	});
	it("formats megabytes", () => {
		expect(formatBytes(5 * 1024 * 1024)).toBe("5.0 MB");
	});
	it("formats gigabytes", () => {
		expect(formatBytes(2.5 * 1024 * 1024 * 1024)).toBe("2.5 GB");
	});
	it("handles null", () => {
		expect(formatBytes(null)).toBe("-");
		expect(formatBytes(undefined)).toBe("-");
	});
});

describe("formatSpeed", () => {
	it("formats speed", () => {
		expect(formatSpeed(3 * 1024 * 1024)).toBe("3.0 MB/s");
	});
	it("handles zero", () => {
		expect(formatSpeed(0)).toBe("-");
	});
});

describe("formatEta", () => {
	it("formats seconds", () => {
		expect(formatEta(42)).toBe("42s");
	});
	it("formats minutes", () => {
		expect(formatEta(125)).toBe("2m 5s");
	});
	it("formats hours", () => {
		expect(formatEta(3725)).toBe("1h 2m");
	});
	it("handles null", () => {
		expect(formatEta(null)).toBe("-");
	});
});

describe("formatDuration", () => {
	it("formats short durations", () => {
		expect(formatDuration(95)).toBe("1:35");
	});
	it("formats long durations", () => {
		expect(formatDuration(3725)).toBe("1:02:05");
	});
	it("handles null", () => {
		expect(formatDuration(null)).toBe("-");
	});
});
