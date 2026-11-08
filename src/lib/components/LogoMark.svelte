<script lang="ts">
	interface Props {
		size?: number;
	}

	let { size = 64 }: Props = $props();
</script>

<svg
	viewBox="0 0 256 256"
	width={size}
	height={size}
	fill="none"
	stroke="currentColor"
	stroke-width="27"
	stroke-linecap="round"
	stroke-linejoin="round"
	aria-hidden="true"
>
	<circle cx="81" cy="85" r="45" />
	<path d="M128 40 V152 Q128 189 91 189 Q62 189 50 163" />
	<path
		d="M88 194 Q184 190 228 92 L238 118 Q198 226 88 222 Z"
		fill="currentColor"
		stroke="none"
	/>
</svg>
