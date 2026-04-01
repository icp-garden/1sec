<script lang="ts">
	import { type Size, isMobile } from '$lib/utils';

	export let size: Size;
</script>

<svg
	viewBox="0 0 24 24"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
	style="width: var(--{size}-icon-size); height: var(--{size}-icon-size);"
>
	<line
		x1="3"
		y1="6"
		x2="21"
		y2="6"
		stroke={'var(--navbar-title-color)'}
		stroke-width="2"
		stroke-linecap="round"
	/>
	<line
		x1="3"
		y1="12"
		x2="21"
		y2="12"
		stroke={'var(--navbar-title-color)'}
		stroke-width="2"
		stroke-linecap="round"
	/>
	<line
		x1="3"
		y1="18"
		x2="21"
		y2="18"
		stroke={'var(--navbar-title-color)'}
		stroke-width="2"
		stroke-linecap="round"
	/>
</svg>
