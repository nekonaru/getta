<script lang="ts">
	import { onMount } from "svelte";
	import { api, type FollowChannel } from "#lib/api";
	import { toastStore } from "#lib/stores/toast.svelte";
	import { t } from "#lib/i18n.svelte";
	import { formatRelativeTime } from "#lib/format";
	import { downloadsStore } from "#lib/stores/downloads.svelte";

	let follows = $state<FollowChannel[]>([]);
	let loading = $state(true);
	let url = $state("");
	let busy = $state(false);
	let checking = $state<string | null>(null);

	onMount(refresh);

	async function refresh() {
		loading = true;
		try {
			follows = await api.followList();
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		} finally {
			loading = false;
		}
	}

	const looksLikeUrl = (text: string) => /^https?:\/\/.+/i.test(text.trim());

	async function follow() {
		const target = url.trim();
		if (!looksLikeUrl(target) || busy) {
			if (!looksLikeUrl(target) && target.length > 0) {
				toastStore.push(t("follows.invalidUrl"), "error");
			}
			return;
		}
		busy = true;
		try {
			const added = await api.followAdd(target);
			url = "";
			follows = [...follows, added];
			toastStore.push(t("follows.added", { title: added.title }));
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		} finally {
			busy = false;
		}
	}

	async function checkNow(follow: FollowChannel) {
		checking = follow.id;
		try {
			const enqueued = await api.followCheckNow(follow.id);
			toastStore.push(
				enqueued > 0 ? t("follows.queued", { count: enqueued }) : t("follows.none")
			);
			follows = follows.map((f) =>
				f.id === follow.id ? { ...f, last_checked_at: Math.floor(Date.now() / 1000) } : f
			);
			void downloadsStore.start();
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		} finally {
			checking = null;
		}
	}

	async function toggleAuto(follow: FollowChannel) {
		const next = !follow.auto_download;
		try {
			await api.followSetAuto(follow.id, next);
			follows = follows.map((f) => (f.id === follow.id ? { ...f, auto_download: next } : f));
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	async function removeFollow(follow: FollowChannel) {
		if (!window.confirm(t("follows.removeConfirm", { title: follow.title }))) return;
		try {
			await api.followRemove(follow.id);
			follows = follows.filter((f) => f.id !== follow.id);
		} catch (e) {
			toastStore.push(e instanceof Error ? e.message : String(e), "error");
		}
	}

	function checkedLabel(follow: FollowChannel): string {
		if (follow.last_checked_at == null) return t("follows.neverChecked");
		return t("follows.lastChecked", { time: formatRelativeTime(follow.last_checked_at) });
	}
</script>

<div class="page">
	<div class="page-inner">
		<header class="page-head">
			<div>
				<p class="kicker">{t("follows.kicker")}</p>
				<h1 class="page-title">{t("follows.title")}</h1>
			</div>
			<span class="pill mono">{follows.length}</span>
		</header>

		<form class="follow-add card" onsubmit={(e) => { e.preventDefault(); void follow(); }}>
			<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4v16M4 12h16" /></svg>
			<input
				class="follow-input"
				type="url"
				bind:value={url}
				placeholder={t("follows.placeholder")}
				aria-label={t("follows.placeholder")}
				spellcheck="false"
				autocapitalize="none"
				autocorrect="off"
			/>
			<button class="btn btn-accent" type="submit" disabled={busy || !url.trim()}>
				{#if busy}<span class="spinner"></span>{:else}{t("follows.add")}{/if}
			</button>
		</form>

		{#if loading}
			<div class="spinner" role="status"></div>
		{:else if follows.length === 0}
			<div class="empty-state">
				<svg viewBox="0 0 256 256" width="84" height="84" aria-hidden="true"><g fill="none" stroke="var(--accent)" stroke-width="22" stroke-linecap="round" stroke-linejoin="round"><circle cx="128" cy="128" r="34"/><circle cx="128" cy="128" r="80" opacity="0.5"/><circle cx="128" cy="128" r="118" opacity="0.25"/></g></svg>
				<h2 class="empty-title">{t("follows.empty")}</h2>
				<p class="muted">{t("follows.emptyHint")}</p>
			</div>
		{:else}
			<ul class="follow-list">
				{#each follows as follow (follow.id)}
					<li class="follow-row card">
						<span class="follow-avatar" aria-hidden="true">{(follow.title || "?").slice(0, 1).toUpperCase()}</span>
						<div class="follow-meta">
							<p class="follow-title" title={follow.title}>{follow.title}</p>
							<a class="follow-url mono" href={follow.url} target="_blank" rel="noreferrer">{follow.url}</a>
							<p class="follow-checked muted mono">{checkedLabel(follow)}</p>
						</div>
						<div class="follow-actions">
							<label class="auto-toggle">
								<span class="auto-label muted">{t("follows.auto")}</span>
								<button class="switch" role="switch" aria-checked={follow.auto_download} aria-label={t("follows.auto")} onclick={() => toggleAuto(follow)}></button>
							</label>
							<button class="btn btn-sm" disabled={checking === follow.id} onclick={() => checkNow(follow)}>
								{checking === follow.id ? t("follows.checking") : t("follows.checkNow")}
							</button>
							<button class="btn-icon danger" title="Remove" aria-label="Remove" onclick={() => removeFollow(follow)}><svg viewBox="0 0 24 24"><path d="M5 7h14M10 11v6M14 11v6M8 7l1-2h6l1 2" /></svg></button>
						</div>
					</li>
				{/each}
			</ul>
		{/if}
	</div>
</div>

<style>
	.follow-add {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 12px 14px;
		margin-bottom: 16px;
	}

	.follow-add svg {
		width: 20px;
		height: 20px;
		stroke: var(--accent);
		fill: none;
		stroke-width: 2;
		stroke-linecap: round;
		flex: none;
	}

	.follow-input {
		flex: 1;
		min-width: 0;
		border: none;
		background: transparent;
		font: inherit;
		color: inherit;
		outline: none;
	}

	.follow-input::placeholder {
		color: var(--muted);
	}

	.follow-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.follow-row {
		display: flex;
		align-items: center;
		gap: 14px;
		padding: 12px 14px;
	}

	.follow-avatar {
		width: 48px;
		height: 48px;
		flex: none;
		border-radius: var(--r-md);
		background: var(--accent-faint);
		border: 1px solid var(--border-soft);
		display: grid;
		place-items: center;
		font-family: var(--font-display);
		font-weight: 800;
		font-size: 1.2rem;
		color: var(--accent);
	}

	.follow-meta {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
		flex: 1;
	}

	.follow-title {
		margin: 0;
		font-weight: 650;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.follow-url {
		font-size: 10px;
		color: var(--muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		text-decoration: none;
	}

	.follow-checked {
		font-size: 9px;
		margin: 0;
	}

	.follow-actions {
		display: flex;
		align-items: center;
		gap: 10px;
		flex: none;
	}

	.auto-toggle {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.auto-label {
		font-size: 10px;
		max-width: 130px;
		text-align: right;
		line-height: 1.3;
	}

	@media (max-width: 720px) {
		.follow-row {
			flex-wrap: wrap;
		}

		.follow-actions {
			width: 100%;
			justify-content: flex-end;
			border-top: 1px solid var(--border);
			padding-top: 10px;
		}

		.auto-label {
			max-width: none;
		}
	}
</style>
