<script lang="ts">
	import ClockIcon from '$lib/icons/ClockIcon.svelte';
	import { type Token } from '$lib/oneSec/types';
	import { prices } from '$lib/stores';

	let {
		dstAmount,
		srcAmount,
		srcToken,
		dstToken,
		onChange,
		readonly = false
	}: {
		dstAmount: number | undefined;
		srcAmount: number | undefined;
		srcToken: Token;
		dstToken: Token;
		onChange: (amount: number) => void;
		readonly?: boolean;
	} = $props();

	let waitTime = $derived(srcToken === 'ckUSDC' || srcToken === 'ckUSDT' ? '8m' : '30s');
</script>

<div class="container">
	<input
		type="text"
		inputmode="decimal"
		autocomplete="off"
		value={dstAmount}
		oninput={(e) => {
			let raw = (e.target as HTMLInputElement).value;
			const regex = /^\d+([.]\d*)?$/;
			if (!regex.test(raw)) raw = raw.slice(0, -1);
			(e.target as HTMLInputElement).value = raw;
			const amount = isNaN(parseFloat(raw)) ? 0 : parseFloat(raw);
			onChange(amount);
		}}
		placeholder="0.00"
		min="0"
		{readonly}
	/>
	<div class="info-row">
		{#if dstAmount && srcAmount}
			<span class="wait-time"> <ClockIcon /> ~{waitTime} </span>
		{/if}
		<span class="dollar-value">
			{#if dstAmount}
				{@const dollarValue = ($prices.get(dstToken) ?? 0) * dstAmount}
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

	span {
		text-align: end;
		font-size: var(--small-font-size);
		color: var(--c-grey);
		display: flex;
		align-items: center;
		gap: 0.2em;
	}

	.info-row {
		display: flex;
		justify-content: flex-end;
		align-items: center;
		gap: 0.6em;
		min-height: 1.4em;
	}

	.dollar-value {
		font-size: 0.65rem;
		font-weight: 400;
		color: var(--c-grey);
	}

	.wait-time {
		color: var(--c-grey);
		font-size: 0.65rem;
		font-weight: 400;
		display: flex;
		align-items: center;
		gap: 0.2em;
	}

	.container {
		display: flex;
		flex-direction: column;
		height: 100%;
		justify-content: center;
	}
</style>
