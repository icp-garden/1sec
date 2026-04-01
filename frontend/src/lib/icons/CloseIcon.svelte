<script lang="ts">
	import type { Size } from '$lib/utils';

	export let color;
	export let size: Size;
</script>

<svg
	viewBox="0 0 24 24"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
	style="width: var(--{size}-icon-size); height: var(--{size}-icon-size);"
	><g stroke-width="0"></g><g stroke-linecap="round" stroke-linejoin="round"></g><g
		><path
			fill-rule="evenodd"
			clip-rule="evenodd"
			d="M19.207 6.207a1 1 0 0 0-1.414-1.414L12 10.586 6.207 4.793a1 1 0 0 0-1.414 1.414L10.586 12l-5.793 5.793a1 1 0 1 0 1.414 1.414L12 13.414l5.793 5.793a1 1 0 0 0 1.414-1.414L13.414 12l5.793-5.793z"
			fill={color}
		></path></g
	></svg
>
