<script lang="ts">
	import type { Size } from '$lib/utils';

	export let animation: 'none' | 'up' | 'down';
	export let size: Size;
</script>

<svg
	style="width: calc(1.4 * var(--{size}-icon-size)); height: calc(1.4 * var(--{size}-icon-size));"
	xmlns="http://www.w3.org/2000/svg"
	viewBox="0 0 24 25"
	fill="none"
	class:up={animation === 'up'}
	class:down={animation === 'down'}
	><path
		d="M18 14.5L12 20.5L6 14.5M12 19.5V4.5"
		stroke="currentColor"
		stroke-width="2"
		stroke-linecap="round"
		stroke-linejoin="round"
	></path></svg
>

<style>
	.up {
		animation: rotate 400ms cubic-bezier(0.3, 0.7, 0, 1) forwards;
	}

	.down {
		animation: rotate-back 200ms cubic-bezier(0.3, 0.7, 0, 1) forwards;
	}

	@keyframes rotate {
		0% {
			transform: rotate(0deg);
		}
		100% {
			transform: rotate(180deg);
		}
	}

	@keyframes rotate-back {
		0% {
			transform: rotate(180deg);
		}
		100% {
			transform: rotate(0deg);
		}
	}
</style>
