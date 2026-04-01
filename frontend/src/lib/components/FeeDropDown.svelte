<script lang="ts">
	import { tokenToLedgerFee } from '$lib/oneSec/config';
	import type { BridgeSettings } from '$lib/oneSec/settings';
	import type { Asset, Chain, EvmChain, Token } from '$lib/oneSec/types';
	import { prices } from '$lib/stores';
	import { evmAnonymous } from '$lib/user/evmUser';
	import { displayValue } from '$lib/utils';
	import { slide } from 'svelte/transition';

	let {
		token,
		evmChain,
		settings
	}: { token: Token; evmChain: EvmChain; settings: BridgeSettings | undefined } = $props();
</script>

<div class="aligned" transition:slide={{ duration: 250 }}>
	<span class="category"> Protocol </span>
	<span class="value">{settings ? settings.protocolFeeInPercent * 100 + ' %' : '-/-'}</span>
</div>
<div class="aligned" transition:slide={{ duration: 250 }}>
	<span class="category"> Network </span>
	{#await evmAnonymous(evmChain)[0].gasCost()}
		<span class="value">-/-</span>
	{:then gas}
		{#if $prices.get(token) === undefined}
			<span class="value">-/-</span>
		{:else if gas * $prices.get(token)! < 0.01}
			<span class="value">{'~'} $0.01</span>
		{:else}
			<span class="value">${displayValue(gas * $prices.get(token)!, 2)}</span>
		{/if}
	{/await}
</div>
<div class="aligned" transition:slide={{ duration: 250 }}>
	<span class="category"> Ledger </span>
	{#if $prices.get(token) === undefined}
		<span class="value">-/-</span>
	{:else if tokenToLedgerFee(token) * $prices.get(token)! < 0.01}
		<span class="value">{'~'} $0.01 </span>
	{:else}
		<span class="value">
			${displayValue(tokenToLedgerFee(token) * $prices.get(token)!, 2)}
		</span>
	{/if}
</div>

<style>
	.category {
		color: var(--c-grey);
	}

	.value {
		color: var(--c-text);
		font-family: 'IBM Plex Mono', monospace;
		font-weight: 500;
	}

	.aligned {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.aligned span {
		font-size: var(--small-font-size);
		margin-top: 0.8em;
		display: flex;
		align-items: center;
		gap: 0.5em;
	}
</style>
