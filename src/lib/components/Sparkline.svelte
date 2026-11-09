<script lang="ts">
	interface Props {
		samples: number[];
		width?: number;
		height?: number;
	}

	let { samples, width = 148, height = 34 }: Props = $props();

	const points = $derived.by(() => {
		if (samples.length < 2) return "";
		const max = Math.max(...samples, 1);
		return samples
			.map((s, i) => {
				const x = (i / (samples.length - 1)) * width;
				const y = height - (s / max) * (height - 3) - 1;
				return `${x.toFixed(1)},${y.toFixed(1)}`;
			})
			.join(" ");
	});
</script>

{#if samples.length >= 2}
	<svg viewBox="0 0 {width} {height}" width={width} height={height} aria-hidden="true" class="sparkline">
		<polyline points={points} fill="none" stroke="var(--accent)" stroke-width="1.6" stroke-linejoin="round" stroke-linecap="round" />
		<polygon points="{points} {width},{height} 0,{height}" fill="var(--accent)" opacity="0.09" stroke="none" />
	</svg>
{/if}

<style>
	.sparkline {
		display: block;
		overflow: visible;
	}
</style>
