<script lang="ts">
	import { openExternal } from "#lib/external";
	import LogoMark from "#lib/components/LogoMark.svelte";
	import { t } from "#lib/i18n.svelte";

	const options = [
		{
			id: "github",
			label: () => t("support.github"),
			desc: () => t("support.githubDesc"),
			url: "https://github.com/sponsors/nekonaru"
		},
		{
			id: "saweria",
			label: () => t("support.saweria"),
			desc: () => t("support.saweriaDesc"),
			url: "https://saweria.co/nekonaruu"
		},
		{
			id: "sociabuzz",
			label: () => t("support.sociabuzz"),
			desc: () => t("support.sociabuzzDesc"),
			url: "https://sociabuzz.com/nekonaru/donate"
		},
		{
			id: "bagibagi",
			label: () => t("support.bagibagi"),
			desc: () => t("support.bagibagiDesc"),
			url: "https://bagibagi.co/Nekonaru"
		}
	] as const;
</script>

<div class="page">
	<div class="page-inner">
		<header class="page-head">
			<div>
				<p class="kicker">{t("support.kicker")}</p>
				<h1 class="page-title">{t("support.title")}</h1>
			</div>
			<LogoMark size={40} />
		</header>

		<p class="intro muted">{t("support.intro")}</p>

		<div class="support-grid">
			{#each options as option (option.id)}
				<button class="support-card" onclick={() => openExternal(option.url)}>
					<span class="card-glyph" aria-hidden="true">{option.label().slice(0, 1)}</span>
					<span class="card-body">
						<span class="card-name">{option.label()}</span>
						<span class="card-desc muted">{option.desc()}</span>
					</span>
					<span class="card-open mono">{t("support.open")} →</span>
				</button>
			{/each}
		</div>

		<p class="fineprint muted">{t("about.license")}</p>
	</div>
</div>

<style>
	.intro {
		margin: 0 0 18px;
		font-size: 13px;
		max-width: 55ch;
	}

	.support-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
		gap: 12px;
	}

	.support-card {
		display: grid;
		grid-template-columns: 48px 1fr auto;
		align-items: center;
		gap: 12px;
		border: 1px solid var(--border);
		background: var(--card);
		border-radius: var(--r-md);
		padding: 14px;
		cursor: pointer;
		text-align: left;
		color: inherit;
		transition: transform 0.18s ease, border-color 0.15s ease, background 0.15s ease;
		box-shadow: var(--shadow);
	}

	.support-card:hover {
		transform: translateY(-2px);
		border-color: var(--border-strong);
		background: var(--raised);
	}

	.support-card:active {
		transform: translateY(0) scale(0.985);
	}

	.card-glyph {
		width: 48px;
		height: 48px;
		background: var(--accent);
		color: var(--on-accent);
		border-radius: var(--r-md);
		display: grid;
		place-items: center;
		font-family: var(--font-display);
		font-weight: 800;
		font-size: 1.4rem;
	}

	.card-body {
		display: flex;
		flex-direction: column;
		gap: 3px;
		min-width: 0;
	}

	.card-name {
		font-weight: 700;
		font-size: 14px;
	}

	.card-desc {
		font-size: 11.5px;
	}

	.card-open {
		font-size: 10px;
		letter-spacing: 0.14em;
		text-transform: uppercase;
		color: var(--accent);
		white-space: nowrap;
	}

	.fineprint {
		font-size: 11px;
		margin-top: 18px;
	}
</style>
