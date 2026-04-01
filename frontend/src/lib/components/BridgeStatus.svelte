<script lang="ts">
	import { slide } from 'svelte/transition';
	import ErrorIcon from '$lib/icons/ErrorIcon.svelte';
	import { bridge, clock } from '$lib/stores';
	import ClockIcon from '$lib/icons/ClockIcon.svelte';
	import type { Tx } from '../../declarations/one_sec/one_sec.did';
	import type { Chain } from '$lib/oneSec/types';

	let { onClose }: { onClose: () => void } = $props();

	function short(err: string): string {
		const index = err.indexOf(':');
		if (index > 0) return err.substring(0, index);
		return err;
	}

	let steps = $derived($bridge?.steps() ?? []);
	let currentStep = $derived(steps.find((s) => s.status.tag !== 'ok') ?? steps[steps.length - 1]);
	let completedSteps = $derived(steps.filter((s) => s.status.tag === 'ok').length);
	let progress = $derived(steps.length > 0 ? (completedSteps / steps.length) * 100 : 0);
	let isDone = $derived($bridge?.done() ?? false);
	let isError = $derived(currentStep?.status.tag === 'err');

	function txLink(chain: Chain, tx?: Tx): string | undefined {
		if (!tx) {
			return undefined;
		}
		switch (true) {
			case 'Icp' in tx: {
				return `https://dashboard.internetcomputer.org/tokens/${tx.Icp.ledger.toString()}/transaction/${tx.Icp.block_index.toString()}`;
			}
			case 'Evm' in tx: {
				switch (chain) {
					case 'Base':
						return `https://basescan.org/tx/${tx.Evm.hash}`;
					case 'Arbitrum':
						return `https:/arbiscan.io/tx/${tx.Evm.hash}`;
					case 'Ethereum':
						return `https://etherscan.io/tx/${tx.Evm.hash}`;
				}
				break;
			}
			default: {
				const _exhaustiveCheck: never = tx;
				throw `unexpected candid tx: ${tx}`;
			}
		}
	}
</script>

{#if $bridge && currentStep}
	<div class="status-container" in:slide>
		{#if isDone}
			<!-- Done state -->
			<div class="done-state">
				{#if currentStep.tx && txLink(currentStep.chain, currentStep.tx)}
					<a href={txLink(currentStep.chain, currentStep.tx)} target="_blank" class="done-link">
						Transferred in {$clock.toFixed(0)}s — view transaction
					</a>
				{:else}
					<span class="done-text">Transferred in {$clock.toFixed(0)}s</span>
				{/if}
			</div>
			{#if currentStep.refund}
				<div class="error-pill">
					Liquidity of {$bridge.request.evmToken} has decreased on {$bridge.request.evmChain}.
					Please retry.
				</div>
			{/if}
			<button class="close-btn" onclick={onClose}>Close</button>
		{:else if isError}
			<!-- Error state -->
			<div class="current-step">
				<span class="step-label error-text">{short(currentStep.label)}</span>
			</div>
			{#if currentStep.label !== short(currentStep.label)}
				<div class="error-pill">
					{currentStep.label.substring(currentStep.label.indexOf(':') + 2)}
				</div>
			{/if}
			<button class="close-btn" onclick={onClose}>Close</button>
		{:else}
			<!-- In progress -->
			<div class="top-row">
				<div class="progress-bar">
					<div class="progress-fill" style="width: {progress}%"></div>
				</div>
				<span class="elapsed">
					<ClockIcon />
					{$clock.toFixed(0)}s
				</span>
			</div>
			<div class="current-step">
				<div class="spinner"></div>
				{#if currentStep.tx && txLink(currentStep.chain, currentStep.tx)}
					<a href={txLink(currentStep.chain, currentStep.tx)} target="_blank" class="step-label"
						>{currentStep.label}</a
					>
				{:else}
					<span class="step-label">{currentStep.label}</span>
				{/if}
			</div>
		{/if}
	</div>
{/if}

<style>
	.status-container {
		display: flex;
		flex-direction: column;
		gap: 0.5em;
		padding: 0.85em 0.9em;
		margin: 0;
		background: color-mix(in srgb, var(--c-text) 3%, var(--c-bg));
		border: var(--s-line) solid color-mix(in srgb, var(--c-text) 8%, var(--c-bg));
		border-radius: 3px;
		box-sizing: border-box;
	}

	/* Top row: bar + clock */
	.top-row {
		display: flex;
		align-items: center;
		gap: 0.6em;
	}

	.progress-bar {
		flex: 1;
		height: 3px;
		background: color-mix(in srgb, var(--c-text) 10%, var(--c-bg));
		border-radius: 2px;
		overflow: hidden;
	}

	.progress-fill {
		height: 100%;
		background: var(--c-text);
		border-radius: 2px;
		transition: width 0.5s ease;
	}

	.progress-fill.error {
		background: var(--c-red);
	}

	.elapsed {
		display: flex;
		align-items: center;
		gap: 0.2em;
		font-family: 'IBM Plex Mono', monospace;
		font-size: 0.55rem;
		font-weight: 400;
		color: var(--c-grey);
		white-space: nowrap;
		min-width: 2.5em;
		justify-content: flex-end;
	}

	.elapsed :global(svg) {
		color: var(--c-grey);
	}

	/* Current step */
	.current-step {
		display: flex;
		align-items: center;
		gap: 0.4em;
	}

	.step-label {
		font-size: 0.65rem;
		font-weight: 400;
		color: var(--c-text);
	}

	.error-text {
		color: var(--c-red);
	}

	.done-state {
		text-align: center;
		padding: 0.4em 0;
	}

	.done-text {
		font-size: 0.65rem;
		font-weight: 500;
		color: var(--c-text);
	}

	.done-link {
		font-size: 0.65rem;
		font-weight: 500;
		color: var(--c-text);
		text-decoration: underline;
		text-underline-offset: 0.15em;
	}

	.done-link:hover {
		opacity: 0.7;
	}

	a.step-label {
		text-decoration: underline;
		text-underline-offset: 0.15em;
		color: var(--c-text);
		cursor: pointer;
	}

	a.step-label:hover {
		opacity: 0.7;
	}

	.spinner {
		width: var(--spinner-size);
		height: var(--spinner-size);
		border-color: var(--c-text);
		border-top-color: transparent;
	}

	.close-btn {
		background: var(--c-text) !important;
		color: var(--c-bg) !important;
		border: none !important;
		border-radius: 2px;
		padding: 0.6em 1em;
		font-weight: 600;
		font-size: 0.75rem;
		font-family: 'FK Roman Standard', system-ui, serif;
		letter-spacing: 0.02em;
		cursor: pointer;
		transition: opacity 0.2s;
		width: 100%;
		text-align: center;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.close-btn:hover {
		opacity: 0.85;
	}

	.error-pill {
		font-family: 'IBM Plex Mono', monospace;
		font-size: 0.55rem;
		font-weight: 400;
		color: var(--c-red);
		background: color-mix(in srgb, var(--c-red) 6%, var(--c-bg));
		border: var(--s-line) solid color-mix(in srgb, var(--c-red) 12%, var(--c-bg));
		border-radius: 2px;
		padding: 0.4em 0.7em;
		word-break: break-word;
	}
</style>
