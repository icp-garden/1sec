<script lang="ts">
	import type { Size } from '$lib/utils';

	export let color: 'black' | 'white';
	export let size: Size;
</script>

<svg
	viewBox="0 0 20 20"
	xmlns="http://www.w3.org/2000/svg"
	fill="none"
	style="width: var(--{size}-icon-size); height: var(--{size}-icon-size);"
	><g stroke-width="0"></g><g stroke-linecap="round" stroke-linejoin="round"></g><g>
		<path
			fill={color}
			fill-rule="evenodd"
			d="M10 3a7 7 0 100 14 7 7 0 000-14zm-9 7a9 9 0 1118 0 9 9 0 01-18 0zm14 .069a1 1 0 01-1 1h-2.931V14a1 1 0 11-2 0v-2.931H6a1 1 0 110-2h3.069V6a1 1 0 112 0v3.069H14a1 1 0 011 1z"
		></path>
	</g></svg
>
