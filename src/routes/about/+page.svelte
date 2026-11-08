<script lang="ts">
	import LogoMark from "#lib/components/LogoMark.svelte";
	import { t } from "#lib/i18n";
	const version = "0.1.0";
</script>

<div class="page">
	<div class="page-inner about-inner">
		<div class="about-mark" aria-hidden="true">
			<LogoMark size={72} />
		</div>
		<h1 class="about-name">Getta</h1>
		<p class="about-tagline">{t("about.tagline")}</p>

		<div class="card about-card">
			<p>{t("about.body")}</p>
			<p class="muted">{t("about.engine")}</p>

			<dl class="about-meta">
				<div><dt>Version</dt><dd class="mono">v{version}</dd></div>
				<div><dt>Identifier</dt><dd class="mono">com.getta.app</dd></div>
				<div><dt>License</dt><dd>MIT</dd></div>
			</dl>

			<nav class="about-links">
				<a href="https://github.com/nekonaru/getta" target="_blank" rel="noreferrer">{t("about.source")}</a>
				<a href="https://github.com/nekonaru/getta/releases" target="_blank" rel="noreferrer">{t("about.releases")}</a>
				<a href="https://github.com/nekonaru/getta/issues/new" target="_blank" rel="noreferrer">{t("about.report")}</a>
			</nav>
			<p class="muted license-line">{t("about.license")}</p>
		</div>
	</div>
</div>

<style>
	.about-inner {
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		gap: 6px;
		padding-top: clamp(32px, 10vh, 72px);
	}

	.about-mark {
		width: 104px;
		height: 104px;
		border-radius: 28px;
		background: var(--spruce);
		display: grid;
		place-items: center;
		color: var(--accent);
		margin-bottom: 10px;
	}

	.about-name {
		font-family: var(--font-display);
		font-size: 2.6rem;
		font-weight: 800;
		letter-spacing: -0.02em;
	}

	.about-tagline {
		color: var(--muted);
		margin: 0 0 12px;
	}

	.about-card {
		padding: 22px 26px;
		max-width: 560px;
		text-align: left;
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.about-card p {
		margin: 0;
	}

	.about-meta {
		display: flex;
		gap: 28px;
		margin: 0;
		flex-wrap: wrap;
		border-top: 1px solid var(--border);
		padding-top: 14px;
	}

	.about-meta dt {
		font-size: 11px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.07em;
		color: var(--muted);
		margin-bottom: 2px;
	}

	.about-meta dd {
		margin: 0;
		font-size: 13px;
	}

	.about-links {
		display: flex;
		gap: 16px;
		flex-wrap: wrap;
	}

	.about-links a {
		color: var(--accent);
		font-weight: 600;
		font-size: 13.5px;
		text-decoration: none;
	}

	.about-links a:hover {
		text-decoration: underline;
	}

	.license-line {
		font-size: 12px;
	}
</style>
