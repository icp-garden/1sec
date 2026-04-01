<script lang="ts">
	import type { Size } from '$lib/utils';

	export let down: boolean;
	export let size: Size;
	export let color: 'white' | 'black';
	export let animation: 'none' | 'up' | 'down';
</script>

<svg
	style="width: var(--{size}-icon-size); height: var(--{size}-icon-size);"
	viewBox="0 0 24 25"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
	class:up={animation === 'up'}
	class:down={animation === 'down'}
>
	<path
		d="M7 9.20337L12 15.2034L17 9.20337"
		stroke={color}
		stroke-width="2"
		stroke-linecap="round"
		stroke-linejoin="round"
		transform={down ? 'rotate(0)' : 'rotate(180, 12, 12)'}
	/>
</svg>

<style>
	.up {
		animation: rotate 200ms cubic-bezier(0.3, 0.7, 0, 1) forwards;
	}

	.down {
		animation: rotate 200ms cubic-bezier(0.3, 0.7, 0, 1) forwards;
	}

	@keyframes rotate {
		0% {
			transform: rotate(180deg);
		}
		100% {
			transform: rotate(0deg);
		}
	}
</style>
