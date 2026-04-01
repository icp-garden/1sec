<script lang="ts">
	import CheckIcon from '$lib/icons/CheckIcon.svelte';
	import CopyIcon from '$lib/icons/CopyIcon.svelte';
	import { showingModalDialog } from '$lib/stores';
	import { truncateAddress, type Size } from '$lib/utils';

	export let showWalletMenu: boolean = false;
	export let triggerWalletMenu = false;
	export let address: string;
	export let imgSrc: string | undefined = '';
	export let short: boolean = false;
	export let style: string = '';
	export let color: 'white' | 'black' = 'black';
	export let allowCopy: boolean = true;
	export let size: Size;

	let copied = false;
</script>

{#if address}
	<button
		{style}
		class="main-btn"
		class:nav-bar-address={triggerWalletMenu}
		on:click={(e) => {
			if (e.target && (e.target as Element).closest('.copy-btn')) {
				e.stopPropagation();
				navigator.clipboard.writeText(address);
				copied = true;
				setTimeout(() => (copied = false), 800);
				return;
			}
			if (triggerWalletMenu) {
				showWalletMenu = !showWalletMenu;
				$showingModalDialog = showWalletMenu;
			}
		}}
	>
		{#if imgSrc && imgSrc.length > 0}
			<img src={imgSrc} alt="" />
		{/if}
		<span
			style:font-size="var(--{size}-font-size)"
			style:color
			style:white-space={short ? 'nowrap' : 'inherit'}
		>
			{short ? truncateAddress(address) : address}
		</span>
		{#if allowCopy}
			<div class="copy-btn">
				{#if copied}
					<CheckIcon {color} {size} />
				{:else}
					<CopyIcon {color} {size} />
				{/if}
			</div>
		{/if}
	</button>
{/if}

<style>
	button {
		color: inherit;
		display: inline-flex;
		align-items: center;
		gap: 0.5em;
		box-sizing: border-box;
		cursor: default;
		background: transparent !important;
		border: none !important;
		box-shadow: none !important;
	}

	span {
		overflow-wrap: anywhere;
		text-align: start;
		font-family: 'IBM Plex Mono', monospace;
		font-weight: 400;
	}

	.copy-btn {
		transition: all 0.3s ease;
		padding: 0;
		display: flex;
	}

	.copy-btn:hover {
		opacity: 0.6;
		cursor: pointer;
	}

	.nav-bar-address {
		background: transparent !important;
		border: none !important;
		padding: 0.3em 0.4em;
		display: flex;
		align-items: center;
		gap: 0.3rem;
		font-family: 'IBM Plex Mono', monospace;
		font-size: 0.55em;
		font-weight: 400;
		color: var(--c-grey) !important;
		box-shadow: none;
	}

	.nav-bar-address:hover {
		background: color-mix(in srgb, var(--c-text) 5%, transparent) !important;
	}

	img {
		width: 1.2rem;
		height: 1.2rem;
	}
</style>
