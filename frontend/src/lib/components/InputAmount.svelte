<script lang="ts">
	import { tokenToLedgerFee } from '$lib/oneSec/config';
	import type { Chain, Token } from '$lib/oneSec/types';
	import { user, prices } from '$lib/stores';

	let {
		onChange,
		chain,
		token,
		srcAmount,
		readonly
	}: {
		onChange: (amount: number) => void;
		chain: Chain;
		token: Token;
		srcAmount: number | undefined;
		readonly: boolean;
	} = $props();

	let balance: number | undefined = $derived($user.getBalance(chain, token)?.amount);

	function shortBalance(v: number): string {
		if (v >= 1000) return v.toFixed(0);
		if (v >= 100) return v.toFixed(1);
		if (v >= 1) return v.toFixed(2);
		if (v >= 0.01) return v.toFixed(3);
		return v.toPrecision(2);
	}
</script>

<div class="container">
	<input
		type="text"
		inputmode="decimal"
		autocomplete="off"
		oninput={(e) => {
			let raw = (e.target as HTMLInputElement).value;
			const regex = /^\d+([.]\d*)?$/;
			if (!regex.test(raw)) raw = raw.slice(0, -1);
			(e.target as HTMLInputElement).value = raw;
			const amount = isNaN(parseFloat(raw)) ? 0 : parseFloat(raw);
			onChange(amount);
		}}
		value={srcAmount}
		placeholder="0.00"
		min="0"
		{readonly}
	/>

	<div class="info-row">
		{#if !readonly && $user.isConnected() && balance !== undefined && balance > 0}
			<button
				class="balance-btn"
				onclick={() => {
					const fee = chain === 'ICP' ? tokenToLedgerFee(token) : 0;
					onChange(Math.max((balance ?? 0) - fee, 0));
				}}
			>
				{shortBalance(balance ?? 0)}
				{token}
			</button>
		{/if}
		<span class="dollar-value">
			{#if srcAmount}
				{@const dollarValue = ($prices.get(token) ?? 0) * srcAmount}
				{#if dollarValue < 0.01}
					$0.00
				{:else}
					${dollarValue.toFixed(2)}
				{/if}
			{:else}
				$0.00
			{/if}
		</span>
	</div>
</div>

<style>
	input[type='text']::-webkit-inner-spin-button,
	input[type='text']::-webkit-outer-spin-button {
		-webkit-appearance: none;
		margin: 0;
	}

	.container {
		display: flex;
		flex-direction: column;
		width: 100%;
		height: 100%;
		justify-content: center;
	}

	input {
		border: none;
		font-family: inherit;
		font-size: 1.4rem;
		font-weight: 450;
		outline: none;
		text-align: right;
		width: 100%;
		padding: 0.15em 0;
		box-sizing: border-box;
		background: transparent;
		color: var(--c-text);
		box-shadow: none;
		letter-spacing: -0.02em;
		min-width: 0;
		min-height: 2.5rem;
	}

	@media (max-width: 530px) {
		input {
			font-size: 1.7rem;
			min-height: 3rem;
			padding: 0.25em 0;
		}
	}

	input::placeholder {
		color: color-mix(in srgb, var(--c-text) 18%, var(--c-bg));
	}

	input:focus {
		box-shadow: none;
	}

	.info-row {
		display: flex;
		justify-content: flex-end;
		align-items: center;
		gap: 0.6em;
		min-height: 1.4em;
	}

	.balance-btn {
		font-size: 0.6rem;
		font-weight: 450;
		color: var(--c-grey);
		white-space: nowrap;
		background: color-mix(in srgb, var(--c-text) 5%, var(--c-bg)) !important;
		border: none !important;
		border-radius: 3px;
		padding: 0.2em 0.6em;
		cursor: pointer;
		transition: color 0.15s;
		font-family: 'IBM Plex Mono', monospace;
	}

	.balance-btn:hover {
		color: var(--c-text);
	}

	.dollar-value {
		font-size: 0.65rem;
		font-weight: 400;
		color: var(--c-grey);
	}
</style>
