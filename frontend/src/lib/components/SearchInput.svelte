<script lang="ts">
	import { page } from '$app/stores';

	let value: string = '';
	let selectedAddress = '';

	function search() {
		selectedAddress = value;
		$page.url.searchParams.set('address', value);
		value = '';
	}
</script>

<div class="input-container">
	<input
		type="text"
		placeholder="Search wallet address"
		bind:value
		spellcheck="false"
		on:keydown={(e) => {
			if (e.key === 'Enter') search();
		}}
	/>
	<a href="/explorer/?address={selectedAddress}" on:click={search} class="search-btn"> Search </a>
</div>

<style>
	input {
		border: none;
		background: transparent;
		padding: 0.5em 0.75em;
		box-sizing: border-box;
		flex: 1;
		font-size: 0.7rem;
		font-family: 'IBM Plex Mono', monospace;
		font-weight: 400;
		color: var(--c-text);
		min-width: 0;
	}

	input::placeholder {
		color: var(--c-grey);
	}

	input:focus {
		outline: none;
		box-shadow: none;
	}

	.input-container {
		display: flex;
		align-items: center;
		background: var(--c-bg);
		border: var(--s-line) solid color-mix(in srgb, var(--c-text) 18%, var(--c-bg));
		border-radius: 2px;
		overflow: hidden;
		transition: border-color 0.15s;
	}

	.input-container:focus-within {
		border-color: var(--c-text);
	}

	.search-btn {
		font-size: 0.6rem;
		font-weight: 550;
		color: var(--c-bg);
		background: var(--c-text);
		border: none;
		padding: 0.5em 1em;
		cursor: pointer;
		text-decoration: none;
		white-space: nowrap;
		border-radius: 2px;
		margin: 0.2em;
		transition: opacity 0.15s;
	}

	.search-btn:hover {
		opacity: 0.85;
		text-decoration: none;
	}
</style>
