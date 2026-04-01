<script lang="ts">
	import { SUPPORTED_ON_ICP, tokenLogoPath } from '$lib/oneSec/config';
	import { Asset } from '$lib/oneSec/types';
	import DownIcon from '$lib/icons/DownIcon.svelte';
	import { slide } from 'svelte/transition';
	import { cubicInOut } from 'svelte/easing';

	export let value = new Asset('ICP', 'ICP');
	let showOptions = false;
</script>

<div class="container">
	<button
		class="asset-btn"
		on:click={() => {
			showOptions = !showOptions;
		}}
	>
		<img class="token-logo" src={tokenLogoPath(value.token)} alt="Logo" />
		<span class="info-token">{value.token}</span>
		<DownIcon down={!showOptions} size="small" color="black" animation="none" />
	</button>
	{#if showOptions}
		<div class="options-container" transition:slide={{ duration: 200, easing: cubicInOut }}>
			{#each SUPPORTED_ON_ICP as asset}
				<button
					class="option-btn"
					on:click={() => {
						value = asset;
						showOptions = false;
					}}
				>
					<img class="token-logo-sm" src={tokenLogoPath(asset.token)} alt="Logo" />
					<span>{asset.token}</span>
				</button>
			{/each}
		</div>
	{/if}
</div>

<style>
	.container {
		display: flex;
		flex-direction: column;
		position: relative;
	}

	.asset-btn {
		display: flex;
		align-items: center;
		gap: 0.35em;
		background: transparent !important;
		border: none !important;
		cursor: pointer;
		padding: 0.2em 0;
	}

	.asset-btn:hover {
		opacity: 0.7;
	}

	.info-token {
		font-weight: 650;
		font-size: 0.75rem;
		color: var(--c-text);
	}

	.token-logo {
		width: 1.4em;
		height: 1.4em;
		border-radius: 50%;
	}

	.token-logo-sm {
		width: 1.5em;
		height: 1.5em;
		border-radius: 50%;
	}

	.options-container {
		display: flex;
		flex-direction: column;
		border-radius: 2px;
		border: var(--s-line) solid color-mix(in srgb, var(--c-text) 10%, var(--c-bg));
		padding: 0.3rem;
		position: absolute;
		top: 2.2em;
		left: 0;
		background: var(--c-bg);
		box-shadow: 0 4px 16px color-mix(in srgb, var(--c-text) 8%, transparent);
		z-index: 10;
		min-width: 8rem;
	}

	.option-btn {
		display: flex;
		align-items: center;
		gap: 0.5em;
		background: transparent !important;
		border: none !important;
		border-radius: 2px;
		padding: 0.6em 0.7em;
		font-size: 0.8rem;
		font-weight: 550;
		color: var(--c-text) !important;
		cursor: pointer;
		white-space: nowrap;
		min-height: 2.5rem;
	}

	.option-btn:hover {
		background: color-mix(in srgb, var(--c-text) 5%, var(--c-bg)) !important;
	}
</style>
