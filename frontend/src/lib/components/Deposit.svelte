<script lang="ts">
	import {
		chainLogoPath,
		getTxExplorerUrl,
		SUPPORTED_ON_EVM,
		SUPPORTED_ON_ICP,
		TOKEN,
		tokenLogoPath,
		MAINNET_ICP_TOKENS
	} from '$lib/oneSec/config';
	import { oneSecForwarding, type OneSecForwarding } from 'onesec-bridge';
	import { type EvmChain, type Token } from '$lib/oneSec/types';
	import { Principal } from '@dfinity/principal';
	import { QRCode } from '@dfinity/gix-components';
	import { slide } from 'svelte/transition';
	import Address from './Address.svelte';
	import ConnectWallet from './ConnectWallet.svelte';
	import { getWallets } from '$lib/wallet/wallet';
	import { displayValue, isEvmAddressValid, isIcrcAccountValid, truncateAddress } from '$lib/utils';
	import { notifyInterval, toasts } from '$lib/stores';
	import { Toast } from '$lib/toast';
	import { onMount } from 'svelte';
	import { evmAnonymous, fetchBalance } from '$lib/user/evmUser';
	import * as fromCandid from '$lib/oneSec/fromCandid';
	import { DEV, STAGING } from '$lib/env';

	let evmChain = $state<EvmChain>('Ethereum');
	let token = $state<Token>('ICP');

	let hovered_token = $state<number | undefined>(undefined);
	let hovered_chain = $state<number | undefined>(undefined);

	let derivedAddress = $state('');
	let toPrincipal = $state('');
	let connect = $state(false);
	let lastStatus: 'checkingBalance' | 'forwarding' | 'forwarded' | undefined = undefined;

	let forwarding: OneSecForwarding;
	if (DEV) {
		forwarding = oneSecForwarding('Local');
	} else if (STAGING) {
		forwarding = oneSecForwarding('Testnet');
	} else {
		forwarding = oneSecForwarding();
	}

	let isSending = $state(false);

	const handleInput = (e: Event) => {
		const raw = (e.target as HTMLTextAreaElement).value;
		const regex = /[\n ]/g;
		toPrincipal = raw.replace(regex, '');

		updateDerivedAddress();
	};

	const updateDerivedAddress = () => {
		try {
			const receiver = {
				owner: Principal.fromText(toPrincipal)
			};
			forwarding.addressFor(receiver).then((address) => {
				derivedAddress = address;
			});
		} catch (e) {
			derivedAddress = '';
		}
	};

	const validateArgs = () => {
		if (toPrincipal === '') {
			toasts.add(Toast.temporaryWarning('Please provide a principal.'));
			return false;
		}
		if (!isIcrcAccountValid(toPrincipal)) {
			toasts.add(Toast.temporaryWarning('The principal is not valid.'));
			return false;
		}
		if (!isEvmAddressValid(derivedAddress)) {
			toasts.add(Toast.temporaryWarning('The evm address is not valid.'));
			return false;
		}

		if ($notifyInterval) {
			toasts.add(Toast.temporaryWarning('Already trying to forward a token.'));
			return false;
		}
		return true;
	};

	const notifyDeposit = async () => {
		if (!validateArgs()) return;
		try {
			isSending = true;
			const receiver = {
				owner: Principal.fromText(toPrincipal)
			};

			const amount = await fetchBalance(evmChain, token, derivedAddress);

			if (amount === 0) {
				toasts.add(Toast.temporaryWarning('The derived address has no balance.'));
				isSending = false;
				return;
			}

			const result = await forwarding.forwardEvmToIcp(token, evmChain, derivedAddress, receiver);
			const lastTransferId = result.done?.id;

			const interval = setInterval(async () => {
				const result = await forwarding.getForwardingStatus(
					token,
					evmChain,
					derivedAddress,
					receiver
				);

				if (
					result.done?.id &&
					(lastTransferId === undefined || result.done.id !== lastTransferId)
				) {
					toasts.add(
						Toast.success(
							`Tokens received at block index ${result.done.id}`
						)
					);
					clearInterval($notifyInterval);
					lastStatus = undefined;
					notifyInterval.reset();
					isSending = false;
					return;
				} else {
					if (!result.status) {
						toasts.add(Toast.temporaryWarning('Balance not found, try again.'));
						lastStatus = undefined;
						clearInterval($notifyInterval);
						notifyInterval.reset();
						isSending = false;
						return;
					}
					switch (true) {
						case 'CheckingBalance' in result.status:
							if (lastStatus === 'checkingBalance') break;
							toasts.add(Toast.temporarySuccess('Checking your balance...'));
							lastStatus = 'checkingBalance';
							break;
						case 'LowBalance' in result.status:
							toasts.add(
								Toast.temporaryWarning(
									`The minimum amount is ${displayValue(Number(result.status.LowBalance.minAmount))} (current balance: ${result.status.LowBalance.balance}).`
								)
							);
							clearInterval($notifyInterval);
							notifyInterval.reset();
							isSending = false;
							lastStatus = undefined;
							return;
						case 'Forwarding' in result.status:
							if (lastStatus === 'forwarding') break;
							toasts.add(Toast.temporarySuccess('Forwarding transfer...'));
							lastStatus = 'forwarding';
							break;
						case 'Forwarded' in result.status:
							if (lastStatus === 'forwarded') break;
								toasts.add(
								Toast.success(
									`Forwarded transfer at ${truncateAddress(result.status.Forwarded.hash)}`
								)
							);
							toasts.add(Toast.temporarySuccess('Tokens will arrive shortly'));
							isSending = false;
							lastStatus = 'forwarded';
							break;
					}
				}
			}, 2_000);

			notifyInterval.set(interval);
		} catch (e) {
			console.error(e);
		}
	};

	function getMargin(hovered: number | undefined, index: number) {
		const overlap = 0.2;
		const extra = 0.2;
		if (index === 0) return 0;

		if (hovered === undefined) {
			return overlap;
		}
		if (index === hovered && index > 0) {
			return overlap + extra;
		}
		if (index === hovered + 1) {
			return overlap + extra;
		}
		return overlap;
	}
	onMount(() => {});
</script>

{#if connect}
	<ConnectWallet
		wallets={getWallets().filter((w) => {
			console.log(w.name);
			return w.kind === 'icp';
		})}
		on:close={() => (connect = false)}
	/>
{/if}

<div class="deposit-container">
	<!-- Step 1: Derive Address -->
	<div class="step">
		<div class="step-header">
			<span class="step-title">1. Derive Address</span>
		</div>
		<p class="step-desc">Get a unique EVM deposit address derived from your ICP principal.</p>
		<div class="panel">
			<label class="panel-label">Destination Principal</label>
			<textarea
				placeholder="Enter principal"
				bind:value={toPrincipal}
				oninput={handleInput}
				readonly={false}
				spellcheck="false"
			></textarea>
		</div>

		{#if derivedAddress != ''}
			<div class="panel derived-panel" in:slide={{ duration: 150 }} out:slide={{ duration: 150 }}>
				<label class="panel-label">Your Deposit Address</label>
				<div class="derived-content">
					<div class="qr-container">
						<QRCode value={derivedAddress} backgroundColor="transparent"></QRCode>
					</div>
					<div class="derived-info">
						<p class="derived-hint">
							Unique to your principal. Send tokens from any supported chain to this address.
						</p>
						<div class="derived-address-row">
							<Address address={derivedAddress} size="tiny" color="black" />
						</div>
					</div>
				</div>
			</div>
		{/if}
	</div>

	<!-- Step 2: Notify Deposit -->
	<div class="step">
		<div class="step-header">
			<span class="step-title">2. Notify Deposit</span>
		</div>
		<p class="step-desc">
			Once you've sent funds to your deposit address, notify the bridge to forward them to your
			principal.
		</p>
		<div class="panel">
			<div class="select-row">
				<label class="panel-label">Token</label>
				<div class="chip-group">
					{#each SUPPORTED_ON_ICP as asset, k}
						<button
							class="chip"
							class:chip--active={token === asset.token}
							onclick={() => (token = asset.token)}
						>
							<img src={tokenLogoPath(asset.token)} alt={asset.token} class="chip-icon" />
							{asset.token}
						</button>
					{/each}
				</div>
			</div>

			<div class="select-row">
				<label class="panel-label">Chain</label>
				<div class="chip-group">
					{#each new Set(SUPPORTED_ON_EVM.slice()
							.reverse()
							.map((a) => a.chain)) as chain, k}
						<button
							class="chip"
							class:chip--active={evmChain === chain}
							onclick={() => (evmChain = chain)}
						>
							<img src={chainLogoPath(chain)} alt={chain} class="chip-icon" />
							{chain}
						</button>
					{/each}
				</div>
			</div>

			<button class="cta" onclick={notifyDeposit}>
				{#if isSending}
					<div class="spinner"></div>
				{:else}
					Notify Deposit
				{/if}
			</button>
		</div>
	</div>
</div>

<style>
	.deposit-container {
		display: flex;
		flex-direction: column;
		width: 100%;
		max-width: 26rem;
		gap: 2rem;
		padding: 0.35rem;
		margin: 0.5em 0;
	}

	.step {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}

	.step-header {
		padding: 0 0.1em 0.1em;
	}

	.step-title {
		font-size: 0.9rem;
		font-weight: 700;
		color: var(--c-text);
		letter-spacing: -0.02em;
	}

	.step-desc {
		font-size: 0.7rem;
		font-weight: 400;
		color: var(--c-grey);
		padding: 0 0.1em;
		margin: 0 0 0.15em;
		line-height: 1.45;
	}

	.panel {
		background: color-mix(in srgb, var(--c-text) 4%, var(--c-bg));
		border: var(--s-line) solid color-mix(in srgb, var(--c-text) 6%, var(--c-bg));
		border-radius: 3px;
		padding: 0.85rem 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.5em;
	}

	.panel-label {
		font-size: 0.7rem;
		font-weight: 500;
		color: var(--c-grey);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	textarea {
		width: 100%;
		height: 2.5em;
		resize: none;
		overflow: hidden;
		word-break: break-word;
		border: none;
		font-size: 1.1rem;
		font-family: 'IBM Plex Mono', monospace;
		font-weight: 400;
		padding: 0;
		background: transparent;
		color: var(--c-text);
	}

	textarea::placeholder {
		color: color-mix(in srgb, var(--c-text) 18%, var(--c-bg));
	}

	textarea:focus-visible {
		outline: none;
	}

	.derived-panel {
	}

	.derived-content {
		display: flex;
		gap: 0.75em;
		align-items: center;
	}

	.derived-info {
		display: flex;
		flex-direction: column;
		gap: 0.4em;
		flex: 1;
		min-width: 0;
	}

	.derived-hint {
		font-size: 0.55rem;
		font-weight: 400;
		color: var(--c-grey);
		line-height: 1.5;
	}

	.derived-address-row {
		display: flex;
		align-items: center;
		padding: 0.35em 0.5em;
		background: color-mix(in srgb, var(--c-text) 3%, var(--c-bg));
		border: var(--s-line) solid color-mix(in srgb, var(--c-text) 8%, var(--c-bg));
		border-radius: 2px;
		font-family: 'IBM Plex Mono', monospace;
		font-size: 0.55rem;
		word-break: break-all;
	}

	.qr-container {
		width: 7em;
		height: 7em;
		flex-shrink: 0;
	}

	.select-row {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 0.5em;
		padding: 0.25em 0;
	}

	.chip-group {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3em;
	}

	.chip {
		display: flex;
		align-items: center;
		gap: 0.3em;
		padding: 0.35em 0.6em;
		border-radius: 3px;
		font-size: 0.7rem;
		font-weight: 550;
		background: var(--c-bg) !important;
		color: var(--c-text) !important;
		border: var(--s-line) solid color-mix(in srgb, var(--c-text) 10%, var(--c-bg)) !important;
		cursor: pointer;
		transition:
			background 0.15s,
			border-color 0.15s;
	}

	.chip:hover {
		background: color-mix(in srgb, var(--c-text) 6%, var(--c-bg)) !important;
	}

	.chip--active {
		background: var(--c-text) !important;
		color: var(--c-bg) !important;
		border-color: var(--c-text) !important;
	}

	.chip--active:hover {
		background: var(--c-text) !important;
		opacity: 0.85;
	}

	.chip-icon {
		width: 1em;
		height: 1em;
		border-radius: 50%;
	}

	.cta {
		background: var(--c-text) !important;
		color: var(--c-bg) !important;
		border: none !important;
		border-radius: 3px;
		padding: 0.9em 1em;
		font-weight: 650;
		font-size: 0.85rem;
		font-family: 'FK Roman Standard', system-ui, serif;
		letter-spacing: 0.01em;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 100%;
		cursor: pointer;
		transition: opacity 0.2s;
		margin-top: 0.5em;
	}

	.cta:hover {
		opacity: 0.85;
	}

	.spinner {
		width: 8px;
		height: 8px;
		border-color: var(--c-bg);
		border-top-color: transparent;
	}
</style>
