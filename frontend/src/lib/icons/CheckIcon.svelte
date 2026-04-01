<script lang="ts">
	import type { Size } from '$lib/utils';

	export let color: 'black' | 'white' = 'white';
	export let size: Size;
</script>

<svg
	style="width: var(--{size}-icon-size); height: var(--{size}-icon-size);"
	viewBox="0 0 24 24"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
>
	<rect x="1.25" y="1.25" width="21.5" height="21.5" rx="10.75" fill="none" />
	<path
		d="M7 11L11 15L17 9"
		stroke={color}
		stroke-width="1.5"
		stroke-linecap="round"
		stroke-linejoin="round"
	/>
	<rect x="1.25" y="1.25" width="21.5" height="21.5" rx="10.75" stroke={color} stroke-width="1.5" />
</svg>
