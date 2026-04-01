<script lang="ts">
	import type { Size } from '$lib/utils';

	export let size: Size;
</script>

<svg
	style="width: var(--{size}-icon-size); height: var(--{size}-icon-size);"
	viewBox="0 0 24 25"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		d="M7 9.20337L12 15.2034L17 9.20337"
		stroke="black"
		stroke-width="2"
		stroke-linecap="round"
		stroke-linejoin="round"
		transform={'rotate(90 12 12)'}
	/>
</svg>
