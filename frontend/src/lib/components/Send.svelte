<script lang="ts">
	import SendAddressInput from './SendAddressInput.svelte';
	import SendAssetButton from './SendAssetButton.svelte';
	import SendAssetInput from './SendAssetInput.svelte';
	import { user, toasts } from '$lib/stores';
	import { Asset } from '$lib/oneSec/types';
	import {
		accountIdentifierFromHex,
		displayValue,
		numberToBigintScaled,
		validateAddr
	} from '$lib/utils';
	import { CONFIG, TOKEN, getTxExplorerUrl, type TokenConfig } from '$lib/oneSec/config';
	import { Principal } from '@dfinity/principal';
	import { Toast } from '$lib/toast';
	import { writeError } from '$lib/resultHandler';
	import type { _SERVICE as ICRC1 } from '../../declarations/icp_ledger/icp_ledger.did';

	export let isSending: boolean;

	let address = '';
	let asset = new Asset('ICP', 'ICP');
	let fee = 0;
	let config: TokenConfig | undefined;
	let balance: number | undefined;
	let amount: number | undefined;
	let isTransferring = false;

	let error = '';
	let visible = false;

	const icrc1Transfer = async () => {
		isTransferring = true;
		const ledger = $user.icp?.ledger(asset.token);
		if (!ledger || !config || !amount) return;
		try {
			const result = await ledger.icrc1_transfer({
				to: { owner: Principal.fromText(address), subaccount: [] },
				fee: [],
				memo: [],
				from_subaccount: [],
				created_at_time: [],
				amount: numberToBigintScaled(amount, config.decimals)
			});
			if ('Ok' in result) {
				const url = getTxExplorerUrl(asset.chain, asset.token, String(result.Ok));
				toasts.add(
					Toast.success(
						`Successful transfer at <a style="color: var(--c-text--interactive)" href=${url}>block index ${result.Ok}</a>`
					)
				);
			} else {
				toasts.add(Toast.error(writeError(result.Err)));
			}
		} catch (e) {
			console.error(e);
			toasts.add(Toast.error(`${asset.token} transfer failed. Please try again.`));
		}
		isTransferring = false;
		isSending = false;
	};

	const icpTransfer = async () => {
		isTransferring = true;
		const ledger = $user.icp?.ledger('ICP');
		if (!ledger || !config || !amount) return;
		try {
			const result = await (ledger as ICRC1).transfer({
				to: accountIdentifierFromHex(address).toUint8Array(),
				fee: { e8s: 10_000n },
				memo: 0n,
				from_subaccount: [],
				created_at_time: [],
				amount: { e8s: numberToBigintScaled(amount, config.decimals) }
			});

			if ('Ok' in result) {
				const url = getTxExplorerUrl('ICP', 'ICP', String(result.Ok));
				toasts.add(
					Toast.success(
						`Successful transfer at <a style="color: white" href=${url + result.Ok}>block index ${result.Ok}</a>`
					)
				);
			} else {
				toasts.add(Toast.error(writeError(result.Err)));
			}
		} catch (e) {
			console.error(e);
			toasts.add(Toast.error('ICP transfer failed. Please try again.'));
		}
		isTransferring = false;
		isSending = false;
	};

	function handleInput() {
		if (isTransferring) {
			toasts.add(Toast.temporaryWarning('Already processing a transfer.'));
			return;
		}

		if (error) {
			toasts.add(Toast.temporaryWarning(error));
			return;
		}
		if (!amount) {
			toasts.add(Toast.temporaryWarning('Please provide an amount.'));
			return;
		}
		if (!address) {
			toasts.add(Toast.temporaryWarning('Please provide a recipient address.'));
			return;
		}
		const account = validateAddr(address, asset.chain, asset.token);
		if (typeof account === 'string') {
			toasts.add(Toast.temporaryWarning(account));
			return;
		}
		if ('Icp' in account) {
			if ('ICRC' in account.Icp) {
				icrc1Transfer();
			} else if ('AccountId' in account.Icp) {
				icpTransfer();
			} else {
				const _unreachable: never = account.Icp;
				throw Error('unreachable');
			}
		} else {
			toasts.add(Toast.temporaryWarning('Unexpected error: address parsed as an EVM address'));
		}
	}

	$: {
		config = TOKEN.get(asset.token);
		balance = $user.icp?.getBalance(asset.token)?.amount;
	}
	$: fee = config?.ledgerFee ?? 0;
	$: if (address || amount != 0) {
		error = '';
		const account = validateAddr(address, asset.chain, asset.token);
		if (typeof account === 'string') {
			error = account;
		}
		if (amount && amount + fee > ($user.getBalance('ICP', asset.token)?.amount ?? 0)) {
			error = 'Balance too low';
		}
	}

	document.startViewTransition(() => {
		visible = true;
	});
</script>

{#if visible}
	<div class="send-container">
		<h2 class="title">Send {asset.token}</h2>

		<div class="input-section">
			<div class="amount-row">
				<SendAssetButton bind:value={asset} />
				<div class="divider"></div>
				<SendAssetInput bind:value={amount} />
				<button class="max-btn" on:click={() => (amount = Math.max((balance ?? 0) - fee, 0))}
					>max</button
				>
			</div>
			<div class="meta-row">
				<span class="meta">
					Balance: {balance !== undefined ? displayValue(balance) : '-/-'}
					{asset.token}
				</span>
				<span class="meta">Fee: {fee} {asset.token}</span>
			</div>
		</div>

		<div class="input-section">
			<SendAddressInput
				style="background: color-mix(in srgb, var(--c-text) 3%, var(--c-bg)); border: var(--s-line) solid color-mix(in srgb, var(--c-text) 8%, var(--c-bg)); border-radius: 2px; padding: 0.6em 0.75em; font-size: 0.7rem;"
				bind:value={address}
			/>
		</div>

		<button class="confirm-btn" on:click={handleInput}>
			{#if isTransferring}
				<div class="spinner"></div>
			{:else}
				Confirm
			{/if}
		</button>
	</div>
{/if}

<style>
	.title {
		margin: 0;
		font-size: 1rem;
		font-weight: 700;
	}

	.send-container {
		display: flex;
		flex-direction: column;
		box-sizing: border-box;
		gap: 0.75em;
		padding: 0;
		padding-bottom: 2em;
		view-transition-name: slide;
		opacity: 0;
		animation: slide-in 250ms var(--tf-snappy);
		animation-fill-mode: forwards;
	}

	@media (max-width: 530px) {
		.send-container {
			padding-bottom: 3.5em;
		}
	}

	@keyframes slide-in {
		from {
			transform: translateX(5em);
			opacity: 0;
		}
		to {
			transform: translateX(0);
			opacity: 1;
		}
	}

	.input-section {
		display: flex;
		flex-direction: column;
		gap: 0.3em;
	}

	.amount-row {
		display: flex;
		align-items: center;
		background: color-mix(in srgb, var(--c-text) 3%, var(--c-bg));
		border: var(--s-line) solid color-mix(in srgb, var(--c-text) 8%, var(--c-bg));
		border-radius: 2px;
		gap: 0.4em;
		padding: 0.5em 0.75em;
	}

	.divider {
		width: var(--s-line);
		height: 1.2em;
		background: color-mix(in srgb, var(--c-text) 12%, var(--c-bg));
	}

	.max-btn {
		background: transparent !important;
		border: none !important;
		color: var(--c-grey) !important;
		font-size: 0.6rem;
		font-weight: 500;
		cursor: pointer;
		padding: 0.2em 0.4em;
	}

	.max-btn:hover {
		color: var(--c-text) !important;
	}

	.meta-row {
		display: flex;
		justify-content: space-between;
		padding: 0 0.25em;
	}

	.meta {
		font-size: 0.55rem;
		font-family: 'IBM Plex Mono', monospace;
		font-weight: 400;
		color: var(--c-grey);
	}

	.spinner {
		width: var(--spinner-size);
		height: var(--spinner-size);
		border-color: var(--c-btn-text);
		border-top-color: transparent;
	}

	.confirm-btn {
		background: var(--c-text) !important;
		color: var(--c-bg) !important;
		border: none !important;
		border-radius: 2px;
		padding: 0.7em 1em;
		font-weight: 600;
		font-size: 0.8rem;
		font-family: 'FK Roman Standard', system-ui, serif;
		letter-spacing: 0.02em;
		cursor: pointer;
		transition: opacity 0.2s;
		width: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.confirm-btn:hover {
		opacity: 0.85;
	}
</style>
