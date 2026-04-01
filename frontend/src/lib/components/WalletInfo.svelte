<script lang="ts">
	import Address from '$lib/components/Address.svelte';
	import { type Token, chains } from '$lib/oneSec/types';
	import PowerOffIcon from '$lib/icons/PowerOffIcon.svelte';
	import { goto } from '$app/navigation';
	import { createEventDispatcher, onMount } from 'svelte';
	import { user } from '$lib/stores';
	import ReceiptIcon from '$lib/icons/ReceiptIcon.svelte';
	import SendIcon from '$lib/icons/SendIcon.svelte';

	export let isSending: boolean = false;
	let balances: { [k: string]: number } = {};
	const dispatch = createEventDispatcher();

	async function disconnectIcp() {
		if ($user.icp) await $user.icp.wallet.disconnect();
		$user.icp = undefined;

		goto('/');
	}

	async function disconnectEvm() {
		if ($user.evm) await $user.evm.wallet.disconnect();
		localStorage.removeItem('evmWalletRdns');
		$user.evm = undefined;

		goto('/');
	}

	async function totalToken(token: Token) {
		const balances = await Promise.all(chains.map((chain) => $user.getBalance(chain, token)));
		return balances.reduce((acc, balance) => {
			if (balance) {
				return acc + balance.amount;
			} else {
				return acc;
			}
		}, 0);
	}

	onMount(() => {
		$user.tokens().forEach((token) => {
			totalToken(token).then((b) => (balances[token] = b));
		});
	});
</script>

{#if $user.icp}
	<div class="user-info-container">
		<div class="left-container">
			<img src={$user.icp.wallet.icon} alt="Icp wallet icon." />
			<div class="icp-address-container">
				<Address address={$user.icp.principal.toText()} color="black" short={true} size="small" />
				<Address address={$user.icp.accountId.toHex()} color="black" short={true} size="small" />
			</div>
		</div>
		<button class="smart" on:click={() => (isSending = true)}>
			<SendIcon size="small" />
		</button>
		<a
			class="smart"
			href={`/explorer/?address=${$user.icp?.principal.toText()}`}
			on:click={() => dispatch('closingTab')}
		>
			<ReceiptIcon size="small" />
		</a>
		<button class="smart" on:click={disconnectIcp}>
			<PowerOffIcon color="black" size="small" />
		</button>
	</div>
{/if}

{#if $user.evm}
	<div class="user-info-container">
		<div class="left-container">
			<img src={$user.evm.wallet.icon} alt="Evm wallet icon." />
			<Address address={$user.evm.address} color="black" short={true} size="small" />
		</div>
		<a
			class="smart"
			href={`/explorer?address=${$user.evm?.address}`}
			on:click={() => dispatch('closingTab')}
		>
			<ReceiptIcon size="small" />
		</a>
		<button class="smart" on:click={disconnectEvm}>
			<PowerOffIcon color="black" size="small" />
		</button>
	</div>
{/if}

<style>
	img {
		width: var(--large-icon-size);
		height: var(--large-icon-size);
	}

	.user-info-container {
		background: var(--c-bg);
		border: var(--s-line) solid var(--c-border);
		padding: 1em;
		display: flex;
		align-items: center;
		width: 100%;
		min-width: 250px;
		box-sizing: border-box;
		animation: slide-in 300ms var(--tf-snappy);
		animation-fill-mode: forwards;
		animation-delay: 100ms;
		opacity: 0;
		transform: translateX(-5rem);
	}

	.icp-address-container {
		display: flex;
		flex-direction: column;
		align-items: start;
		flex-grow: 1;
	}

	@keyframes slide-in {
		0% {
			opacity: 0;
			transform: translateX(-5rem);
		}
		20% {
			opacity: 0;
		}
		100% {
			opacity: 1;
			transform: translateX(0);
		}
	}

	.left-container {
		font-size: small;
		flex: 1;
		display: flex;
		align-items: center;
		gap: 0.8em;
	}

	.smart {
		display: flex;
		align-items: center;
		padding: 0.5em;
		background: transparent;
		border: none;
		color: var(--c-text);
		border-radius: 50%;
		min-width: 2.2rem;
		min-height: 2.2rem;
		justify-content: center;
	}

	.smart:hover {
		background: color-mix(in srgb, var(--c-text) 8%, transparent);
	}
</style>
