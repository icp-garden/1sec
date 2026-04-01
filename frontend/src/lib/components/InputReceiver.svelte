<script lang="ts">
	import type { Chain, Token } from '$lib/oneSec/types';
	import { user, showLoginSidebar } from '$lib/stores';
	import { slide } from 'svelte/transition';
	import Address from './Address.svelte';
	import { validateAddr } from '$lib/utils';
	import DownIcon from '$lib/icons/DownIcon.svelte';

	export let chain: Chain;
	export let token: Token;
	export let receiveAmount: number | undefined;
	export let selectedAddress: string;
	export let readonly: boolean;

	let senderWalletIcon: string | undefined;
	let enterCustomWallet: boolean = false;
	let customInput: HTMLTextAreaElement;

	let defaultAddress = '';
	let customAddress = '';

	let animation: 'none' | 'up' | 'down' = 'none';

	const selectDefaultWallet = () => {
		if (chain === 'ICP' && $user.icp?.principal !== undefined) {
			defaultAddress = $user.icp.principal.toText();
			senderWalletIcon = $user.icp.wallet.icon;
		} else if (chain !== 'ICP' && $user.evm?.address !== undefined) {
			defaultAddress = $user.evm.address;
			senderWalletIcon = $user.evm.wallet.icon;
		} else {
			defaultAddress = '';
			senderWalletIcon = undefined;
		}
	};

	$: ([chain, $user], selectDefaultWallet());
	$: if (enterCustomWallet) {
		selectedAddress = customAddress;
	} else {
		selectedAddress = defaultAddress;
	}

	$: if (enterCustomWallet && customInput) {
		customInput.focus();
	}
</script>

<div class="input-container">
	<div class="header-container">
		<div class="title-container">
			<h3 class="panel-label">Receive</h3>
		</div>
		{#if $user.icp || $user.evm}
			<div style="display: flex; flex-direction: row; gap: .1rem; align-items: center;">
				{#if !enterCustomWallet && defaultAddress !== '' && typeof validateAddr(defaultAddress, chain, token) !== 'string'}
					<Address
						address={defaultAddress}
						short={true}
						color={'black'}
						triggerWalletMenu={false}
						style="background: none; color: grey; padding: 0; flex: 1; gap: .2rem; font-size: 12px;"
						size="small"
						allowCopy={true}
						imgSrc={senderWalletIcon}
					/>
				{:else if enterCustomWallet && chain === 'ICP' && $user.icp}
					<button
						class="default-wallet"
						on:click={() => {
							selectDefaultWallet();
							enterCustomWallet = false;
							animation = 'up';
						}}
					>
						use {$user.icp?.wallet.name}
					</button>
				{:else if enterCustomWallet && chain !== 'ICP' && $user.evm}
					<button
						class="default-wallet"
						on:click={() => {
							selectDefaultWallet();
							enterCustomWallet = false;
							animation = 'up';
						}}
					>
						use {$user.evm.wallet.name}
					</button>
				{/if}
				<button
					on:click={() => {
						enterCustomWallet = !enterCustomWallet;
						animation = enterCustomWallet ? 'down' : 'up';
					}}
				>
					{#key animation}
						<DownIcon size="normal" color="black" {animation} down={!enterCustomWallet} />
					{/key}
				</button>
			</div>
		{/if}
	</div>
	{#if enterCustomWallet}
		<textarea
			bind:this={customInput}
			in:slide={{ duration: 275 }}
			out:slide={{ duration: 275 }}
			placeholder="Enter custom {chain === 'ICP'
				? token === 'ICP'
					? 'principal or account'
					: 'principal'
				: 'address'}"
			bind:value={customAddress}
			{readonly}
			spellcheck="false"
		></textarea>
	{/if}
</div>

<style>
	.title-container {
		display: flex;
		flex-grow: 1;
		gap: 0.5em;
		align-items: center;
	}

	span {
		font-size: 0.65rem;
		place-content: center;
		color: var(--c-grey);
		font-family: inherit;
		font-weight: 400;
	}

	textarea {
		width: 100%;
		resize: none;
		overflow: hidden;
		word-break: break-word;
		flex-wrap: wrap;
		place-content: center;
		border: none;
		border-bottom: var(--s-line) solid var(--c-border);
		font-size: var(--small-font-size);
		font-family: 'IBM Plex Mono', monospace;
		font-weight: 300;
		margin-bottom: 0.4rem;
		background: transparent;
		color: var(--c-text);
		padding: 0.3em 0;
	}

	textarea:focus-visible {
		outline: none;
		border-bottom-color: var(--c-text);
	}

	button {
		padding: 0.2rem;
		background: transparent !important;
		border: none;
		color: var(--c-text) !important;
	}

	.panel-label {
		font-size: 0.7rem;
		font-weight: 500;
		color: var(--c-grey);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.input-container {
		display: flex;
		flex-direction: column;
		width: 100%;
	}

	.header-container {
		display: flex;
		justify-content: space-between;
		gap: 0.4rem;
	}

	.default-wallet {
		font-size: 0.65rem;
		font-weight: 400;
		text-decoration: underline;
		padding: 0;
		color: var(--c-text--interactive) !important;
		background: transparent !important;
	}

	.connect-btn {
		margin: 0;
		font-size: 0.65rem;
		font-weight: 400;
		background: transparent !important;
		border: none;
		flex: 1;
		text-decoration: underline;
		color: var(--c-text--interactive) !important;
	}

	button:hover {
		opacity: 0.7;
	}
</style>
