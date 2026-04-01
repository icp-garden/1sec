<script lang="ts">
	import type { Chain, Token } from '$lib/oneSec/types';
	import { user, showLoginSidebar } from '$lib/stores';
	import { EvmUser } from '$lib/user/evmUser';
	import { IcpUser } from '$lib/user/icpUser';
	import { slide } from 'svelte/transition';
	import Address from './Address.svelte';
	export let chain: Chain;
	export let token: Token;
	export let sender: string;

	let senderWalletIcon: string;

	$: sender = select(chain, $user.icp, $user.evm);
	$: senderWalletIcon = updateWalletIcon(chain, $user.icp, $user.evm);

	function select(chain: Chain, icpUser?: IcpUser, evmUser?: EvmUser): string {
		if (chain === 'ICP') {
			if (icpUser) {
				return icpUser.principal.toText();
			} else {
				return 'connect wallet';
			}
		} else {
			if (evmUser) {
				return evmUser.address;
			} else {
				return 'connect wallet';
			}
		}
	}

	function updateWalletIcon(chain: Chain, icpUser?: IcpUser, evmUser?: EvmUser): string {
		if (chain === 'ICP') {
			if (icpUser) {
				return icpUser.wallet.icon;
			}
		} else {
			if (evmUser) {
				return evmUser.wallet.icon;
			}
		}
		return '';
	}
</script>

<div class="container">
	<div class="header-container">
		<div class="title-container">
			<h3 class="panel-label">Send</h3>
		</div>
		{#if sender && sender !== 'connect wallet'}
			<div class="address" transition:slide={{ duration: 300 }}>
				<Address
					address={sender}
					short={true}
					color={'black'}
					triggerWalletMenu={false}
					style="background: none; color: grey; padding: 0;"
					size="small"
					allowCopy={true}
					imgSrc={senderWalletIcon}
				/>
			</div>
		{/if}
	</div>
</div>

<style>
	.container {
		display: flex;
		flex-direction: column;
		width: 100%;
		box-sizing: border-box;
	}

	.header-container {
		display: flex;
		align-items: start;
		gap: 0.4rem;
		padding: 0;
		cursor: pointer;
	}

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

	.connect-btn {
		margin: 0;
		font-size: 0.65rem;
		font-weight: 400;
		background: transparent !important;
		border: none;
		padding: 0.3rem 0;
		text-decoration: underline;
		color: var(--c-text--interactive) !important;
	}

	.connect-btn:hover {
		opacity: 0.7;
	}

	.panel-label {
		font-size: 0.7rem;
		font-weight: 500;
		color: var(--c-grey);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.address {
		color: var(--c-grey);
		word-break: break-word;
		margin-top: 0;
		display: flex;
		gap: 0.4em;
		align-items: center;
		font-size: var(--normal-font-size);
		font-family: 'IBM Plex Mono', monospace;
	}
</style>
