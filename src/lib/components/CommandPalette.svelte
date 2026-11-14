<script lang="ts">
	import { goto } from "$app/navigation";
	import { onMount, onDestroy } from "svelte";
	import { api } from "#lib/api";
	import { downloadsStore } from "#lib/stores/downloads.svelte";
	import { settingsStore } from "#lib/stores/settings.svelte";
	import { enqueueFromClipboardText } from "#lib/enqueue";
	import { t } from "#lib/i18n.svelte";

	type ActionGroup = "nav" | "actions";
	interface Action {
		id: string;
		group: ActionGroup;
		label: () => string;
		hint?: string;
		run: () => void | Promise<void>;
	}

	let open = $state(false);
	let query = $state("");
	let selected = $state(0);
	let inputEl = $state<HTMLInputElement | null>(null);

	const actions: Action[] = [
		{ id: "home", group: "nav", label: () => t("palette.goHome"), run: () => goto("/") },
		{ id: "queue", group: "nav", label: () => t("palette.goQueue"), run: () => goto("/queue") },
		{ id: "follows", group: "nav", label: () => t("palette.goFollows"), run: () => goto("/follows") },
		{ id: "library", group: "nav", label: () => t("palette.goLibrary"), run: () => goto("/library") },
		{ id: "tools", group: "nav", label: () => t("palette.goTools"), run: () => goto("/tools") },
		{ id: "settings", group: "nav", label: () => t("palette.goSettings"), run: () => goto("/settings") },
		{ id: "about", group: "nav", label: () => t("palette.goAbout"), run: () => goto("/about") },
		{
			id: "clipboard",
			group: "actions",
			label: () => t("palette.downloadFromClipboard"),
			hint: "Ctrl+Shift+D",
			run: async () => {
				try {
					const text = await api.getClipboardText();
					await enqueueFromClipboardText(text);
				} catch {
					/* clipboard unavailable */
				}
			}
		},
		{
			id: "theme",
			group: "actions",
			label: () => t("palette.toggleTheme"),
			run: () => {
				const current = settingsStore.value?.theme ?? "system";
				const next = current === "system" ? "light" : current === "light" ? "dark" : "system";
				void settingsStore.update({ theme: next }).catch(() => {});
			}
		},
		{
			id: "pause-all",
			group: "actions",
			label: () => t("palette.pauseAll"),
			run: () => api.pauseAll().catch(() => {})
		},
		{
			id: "resume-all",
			group: "actions",
			label: () => t("palette.resumeAll"),
			run: () => api.resumeAll().catch(() => {})
		},
		{
			id: "clear-finished",
			group: "actions",
			label: () => t("palette.clearFinished"),
			run: () => api.clearFinished().catch(() => {})
		}
	];

	/**
	 * Fuzzy subsequence match with scoring: consecutive hits and
	 * word-boundary hits score higher. Returns null when `needle`
	 * is not a subsequence of `haystack`; the score ranks matches.
	 */
	function fuzzyScore(haystack: string, needle: string): number | null {
		const h = haystack.toLowerCase();
		const n = needle.toLowerCase();
		if (!n) return 0;
		let score = 0;
		let hi = 0;
		let streak = 0;
		let prevMatch = -2;
		for (let ni = 0; ni < n.length; ni++) {
			const ch = n[ni];
			let found = -1;
			while (hi < h.length) {
				if (h[hi] === ch) {
					found = hi;
					hi++;
					break;
				}
				hi++;
			}
			if (found === -1) return null;
			if (found === prevMatch + 1) {
				streak += 1;
			} else {
				streak = 0;
			}
			const boundary = found === 0 || /[\s\-_/]/.test(h[found - 1] ?? " ");
			score += 1 + streak * 2 + (boundary ? 3 : 0);
			prevMatch = found;
		}
		return score;
	}

	const filtered = $derived.by(() => {
		const needle = query.trim();
		if (!needle) return actions;
		return actions
			.map((a) => ({ action: a, score: fuzzyScore(a.label(), needle) }))
			.filter((r): r is { action: Action; score: number } => r.score !== null)
			.sort((a, b) => b.score - a.score)
			.map((r) => r.action);
	});

	const navActions = $derived(filtered.filter((a) => a.group === "nav"));
	const actionActions = $derived(filtered.filter((a) => a.group === "actions"));

	function flatIndex(action: Action): number {
		return filtered.indexOf(action);
	}

	function openPalette() {
		open = true;
		query = "";
		selected = 0;
		queueMicrotask(() => inputEl?.focus());
	}

	function closePalette() {
		open = false;
	}

	function runAction(action: Action) {
		closePalette();
		void action.run();
	}

	function handleKeydown(event: KeyboardEvent) {
		const key = event.key.toLowerCase();
		if ((event.metaKey || event.ctrlKey) && key === "k") {
			event.preventDefault();
			if (open) closePalette();
			else openPalette();
			return;
		}
		if (!open) return;
		if (event.key === "Escape") {
			event.preventDefault();
			closePalette();
		} else if (event.key === "ArrowDown") {
			event.preventDefault();
			selected = filtered.length > 0 ? (selected + 1) % filtered.length : 0;
		} else if (event.key === "ArrowUp") {
			event.preventDefault();
			selected = filtered.length > 0 ? (selected - 1 + filtered.length) % filtered.length : 0;
		} else if (event.key === "Enter" && filtered[selected]) {
			event.preventDefault();
			runAction(filtered[selected]);
		}
	}

	onMount(() => window.addEventListener("keydown", handleKeydown));
	onDestroy(() => window.removeEventListener("keydown", handleKeydown));
</script>

{#if open}
	<div
		class="palette-veil"
		role="presentation"
		onclick={(e) => e.target === e.currentTarget && closePalette()}
	>
		<div
			class="palette"
			role="dialog"
			aria-modal="true"
			aria-label={t("palette.placeholder")}
		>
			<input
				bind:this={inputEl}
				class="palette-input"
				type="text"
				bind:value={query}
				placeholder={t("palette.placeholder")}
				spellcheck="false"
				autocomplete="off"
				aria-label={t("palette.placeholder")}
			/>
			<div class="palette-list" role="listbox">
				{#if navActions.length > 0}
					<p class="palette-group">{t("palette.groupNav")}</p>
					{#each navActions as action (action.id)}
						<button class="palette-item" role="option" aria-selected={flatIndex(action) === selected} onclick={() => runAction(action)} onmousemove={() => (selected = flatIndex(action))}>
							<span>{action.label()}</span>
							{#if action.hint}<span class="palette-hint">{action.hint}</span>{/if}
						</button>
					{/each}
				{/if}
				{#if actionActions.length > 0}
					<p class="palette-group">{t("palette.groupActions")}</p>
					{#each actionActions as action (action.id)}
						<button class="palette-item" role="option" aria-selected={flatIndex(action) === selected} onclick={() => runAction(action)} onmousemove={() => (selected = flatIndex(action))}>
							<span>{action.label()}</span>
							{#if action.hint}<span class="palette-hint">{action.hint}</span>{/if}
						</button>
					{/each}
				{/if}
				{#if filtered.length === 0}
					<p class="hint" style="padding: 14px 12px">{t("palette.empty")}</p>
				{/if}
			</div>
		</div>
	</div>
{/if}
