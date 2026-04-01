<script lang="ts">
	import {
		SUPPORTED,
		TOKEN,
		tokenToLedgerFee,
		TRANSFER_FEE_MULTIPLIER
	} from '$lib/oneSec/config';
	import type { BridgeSettings } from '$lib/oneSec/settings';
	import type { BridgeRequest, Contracts, EvmChain, BridgeDirection } from '$lib/oneSec/types';
	import { Asset } from '$lib/oneSec/types';
	import BridgeStatus from './BridgeStatus.svelte';
	import { bridge, bridgeSettings, user, toasts, clock, bridgeRequest } from '$lib/stores';
	import InputAmount from './InputAmount.svelte';
	import InputReceiver from './InputReceiver.svelte';
	import WalletTokenInfo from './WalletTokenInfo.svelte';
	import InputSender from './InputSender.svelte';
	import { Principal } from '@dfinity/principal';
	import * as toCandid from '$lib/oneSec/toCandid';
	import { IcpUser } from '$lib/user/icpUser';
	import { EvmUser } from '$lib/user/evmUser';
	import { bridgeable } from '$lib/oneSec/config';
	import ReceiveAmount from './ReceiveAmount.svelte';
	import { Bridge } from '$lib/oneSec/bridge';
	import InputAssetButton from './InputAssetButton.svelte';
	import { displayValue, validateAddr } from '$lib/utils';
	import { Toast } from '$lib/toast';
	import DownArrowIcon from '$lib/icons/DownArrowIcon.svelte';
	import type { _SERVICE as ICRC2 } from '../../declarations/icrc_ledger/icrc_ledger.did';
	import ClockIcon from '$lib/icons/ClockIcon.svelte';
	import { showLoginSidebar } from '$lib/stores';

	let waitingForBridge: boolean = $state(false);

	let src = $state(new Asset('ICP', 'ICP'));
	let srcAddr = $state('');
	let srcAmount = $state<number | undefined>(undefined);

	let dst: Asset = $state(new Asset('Base', 'ICP'));
	let dstAddr = $state('');
	let dstAmount = $state<number | undefined>(undefined);

	let settings: BridgeSettings | undefined = $state(undefined);

	let dropdown: 'src' | 'dst' | undefined = $state(undefined);

	let downArrowAnimation: 'up' | 'down' | 'none' = $state('none');

	let showConfirmation = $state(false);

	$effect(() => {
		if (waitingForBridge && (!$bridge || $bridge.done())) {
			waitingForBridge = false;
			showConfirmation = false;
		}
	});

	let srcAmountError = $derived(validateAmount(src, dst, srcAmount, settings));
	let dstAddrError = $derived(validateAddr(dstAddr, dst.chain, dst.token));

	const handleFullInput = async (src: Asset, srcAmount: number, dst: Asset, dstAddr: string) => {
		try {
			await prepare(src, srcAmount, dst, dstAddr);
		} catch (e) {
			console.warn(e);
		}
	};

	$effect(() => {
		if (src && srcAmount && dst && dstAddr && $user.isConnected())
			handleFullInput(src, srcAmount, dst, dstAddr);
	});

	const computeDstAmount = (amount: number) => {
		if (!settings) return;

		if (TOKEN.get(src.token)!.decimals != TOKEN.get(dst.token)!.decimals) {
			throw new Error(`Internal error: mismatching decimals for ${src.token} and ${dst.token}`);
		}
		let transferFee = settings.transferFee * TRANSFER_FEE_MULTIPLIER.get(dst.chain)!;
		if (src.token === 'ckUSDC' || src.token === 'ckUSDT') {
			transferFee += 0.88;
		}
		const percentToDeduce = 1 - settings.protocolFeeInPercent;

		return Math.max(amount * percentToDeduce - transferFee, 0);
	};

	const computeSrcAmount = (amount: number) => {
		if (!settings) return;

		if (TOKEN.get(src.token)!.decimals != TOKEN.get(dst.token)!.decimals) {
			throw new Error(`Internal error: mismatching decimals for ${src.token} and ${dst.token}`);
		}
		let transferFee = settings.transferFee * TRANSFER_FEE_MULTIPLIER.get(dst.chain)!;
		if (src.token === 'ckUSDC' || src.token === 'ckUSDT') {
			transferFee += 0.88;
		}

		const percentToDeduce = 1 - settings.protocolFeeInPercent;

		return amount / percentToDeduce + transferFee;
	};

	function swapSrcAndDst() {
		const int = dst;
		dst = src;
		src = int;
		const intAddr = dstAddr;
		dstAddr = srcAddr;
		srcAddr = intAddr;
	}

	function validateAmount(
		src: Asset,
		dst: Asset,
		srcAmount: number | undefined,
		settings?: BridgeSettings
	): string {
		if (!srcAmount) {
			return '';
		}
		if (src.chain === 'ICP' && !$user.icp) {
			return 'Connect your ICP wallet';
		}
		if (src.chain !== 'ICP' && !$user.evm) {
			return `Connect your ${src.chain} wallet`;
		}
		const fee = src.chain === 'ICP' ? tokenToLedgerFee(src.token) : 0;
		if (srcAmount + fee > ($user.getBalance(src.chain, src.token)?.amount ?? 0)) {
			return 'Balance too low';
		}
		if (src.token === 'ckUSDC' || src.token === 'ckUSDT') {
			if (srcAmount < 20) {
				return `Amount of ${src.token} should be at least 20`;
			}
			return '';
		}
		if (!settings || settings.src.key() !== src.key() || settings.dst.key() !== dst.key()) {
			// The settings are not fetched yet.
			return '';
		}
		if (srcAmount < settings.minAmount) {
			return `Amount of ${src.token} should be at least ${settings.minAmount}`;
		}
		if (srcAmount > settings.maxAmount) {
			return `Amount of ${src.token} should be at most ${settings.maxAmount}`;
		}
		return '';
	}

	async function prepare(src: Asset, srcAmount: number, dst: Asset, dstAddr: string) {
		if (dstAddr && typeof validateAddr(dstAddr, dst.chain, dst.token) === 'string') {
			throw 'invalid input';
		}

		const fetched = await $bridgeSettings.get(src, dst);
		if (!fetched) {
			throw new Error('Failed to fetch fees. Please retry later.');
		}

		settings = fetched;

		if (srcAmount && validateAmount(src, dst, srcAmount, settings) != '') {
			throw 'invalid input';
		}

		if (!dstAmount) {
			throw 'waiting for input';
		}

		if (settings.available != undefined && dstAmount > settings.available) {
			throw new Error(
				`Only ${displayValue(settings.available)} ${dst.token} is available on ${dst.chain}.` +
					' Consider splitting the amount between other chains.'
			);
		}

		if (!user) {
			throw new Error('Internal error: sender address does not match sender wallet');
		}
	}

	function start(dstAmount: number) {
		if (!srcAmount) {
			return;
		}
		if (validateAmount(src, dst, srcAmount, settings) != '') {
			return;
		}

		const account = validateAddr(dstAddr, dst.chain, dst.token);
		if (typeof account === 'string') {
			return;
		}

		bridgeRequest.set({
			src: { chain: src.chain, token: src.token, amount: srcAmount, address: srcAddr },
			dst: { chain: dst.chain, token: dst.token, amount: dstAmount, address: dstAddr }
		});

		let request: BridgeRequest;

		if (src.chain === 'ICP') {
			let direction: BridgeDirection = 'IcpToEvm';
			if (src.token === 'ckUSDC' || src.token === 'ckUSDT') {
				direction = 'ckToOneSec';
			}
			request = {
				direction: direction,
				icpAccount: { ICRC: { owner: Principal.fromText(srcAddr), subaccount: [] } },
				icpToken: src.token,
				icpAmount: toCandid.amount(srcAmount, TOKEN.get(src.token)!.decimals),
				evmChain: dst.chain as EvmChain,
				evmAccount: dstAddr,
				evmToken: dst.token,
				evmAmount: toCandid.amount(dstAmount, TOKEN.get(dst.token)!.decimals),
				user: $user.icp!
			};
		} else {
			let icpAccount;
			if ('Icp' in account) {
				icpAccount = account.Icp;
			} else {
				console.error(`unexpected EVM account: ${account}`);
				return;
			}

			request = {
				direction: 'EvmToIcp',
				icpAccount,
				icpToken: dst.token,
				icpAmount: toCandid.amount(dstAmount, TOKEN.get(dst.token)!.decimals),
				evmChain: src.chain as EvmChain,
				evmAccount: srcAddr,
				evmToken: src.token,
				evmAmount: toCandid.amount(srcAmount, TOKEN.get(src.token)!.decimals),
				user: $user.evm!
			};
		}
		if (!$bridge || $bridge.done()) {
			waitingForBridge = true;
			let newBridge = new Bridge(request);
			bridge.set(newBridge);
			setTimeout(async () => {
				const contracts = await getContracts(request);
				await newBridge.run(contracts);
			});
		}
	}

	async function getContracts(request: BridgeRequest): Promise<Contracts> {
		console.log(request);
		let result: Contracts = {};
		if (request.direction === 'IcpToEvm') {
			result.oneSec = (request.user as IcpUser).oneSec();
			result.icrc2 = (request.user as IcpUser).ledger(request.icpToken) as ICRC2;
		} else if (request.direction === 'ckToOneSec') {
			result.ckUnwrap = (request.user as IcpUser).ckUnwrap();
			result.icrc2 = (request.user as IcpUser).ledger(request.icpToken) as ICRC2;
		} else {
			result.evmUser = request.user as EvmUser;
			try {
				await result.evmUser.switchChain(request.evmChain);
			} catch (err) {
				console.warn(err);
			}
			result.erc20 = result.evmUser.erc20(request.evmChain, request.evmToken);
			result.locker = result.evmUser.locker(request.evmChain, request.evmToken);
		}
		console.log(result);
		return result;
	}

	function availableDst(src: Asset): Asset[] {
		return SUPPORTED.filter((x) => bridgeable(src, x));
	}

	const isTransferAvailable = () => {
		if (!$user.isConnected()) {
			return false;
		}
		if (!srcAmount) {
			return false;
		}
		if (!dstAddr) {
			return false;
		}
		if (!dstAmount || !user) {
			return false;
		}
		if (srcAmountError !== '') {
			return false;
		}
		if (typeof dstAddrError === 'string') {
			return false;
		}
		if (settings?.available !== undefined && dstAmount > (settings.available ?? 0)) {
			return false;
		}
		return true;
	};

	function clickSrc() {
		if (waitingForBridge) {
			return;
		}
		if (dropdown != 'src') {
			dropdown = 'src';
		} else {
			dropdown = undefined;
		}
	}

	function clickDst() {
		if (waitingForBridge) {
			return;
		}
		if (dropdown != 'dst') {
			dropdown = 'dst';
		} else {
			dropdown = undefined;
		}
	}

	function newSrc(event: CustomEvent<Asset>) {
		src = event.detail;
		const ds = availableDst(src);
		if (!ds.find((v) => v.chain === dst.chain && v.token == dst.token)) {
			dst = ds[0];
		}
		dropdown = undefined;
	}

	function newDst(event: CustomEvent<Asset>) {
		dst = event.detail;
		dropdown = undefined;
	}

	$effect(() => {
		if ($bridge) {
			if ($bridge.done()) {
				clock.pause();
			} else if ($bridge.currentStep >= (src.chain === 'ICP' ? 1 : 2)) {
				clock.start();
			}
		}
	});

	const updateSettings = async () => {
		settings = await $bridgeSettings.get(src, dst);
	};

	$effect(() => {
		if (src && dst) updateSettings();
	});

	$effect(() => {
		if ($bridgeRequest) {
			srcAddr = $bridgeRequest.src.address;
			srcAmount = $bridgeRequest.src.amount;
			src = new Asset($bridgeRequest.src.chain, $bridgeRequest.src.token);
			dstAddr = $bridgeRequest.dst.address;
			dstAmount = $bridgeRequest.dst.amount;
			dst = new Asset($bridgeRequest.dst.chain, $bridgeRequest.dst.token);
		}
	});
</script>

<div class="bridge-form">
	<div
		class={(waitingForBridge || ($bridge && $bridge.done()) ? 'readonly' : '') +
			' content-container'}
		style="margin-top: 0;"
	>
		<InputSender chain={src.chain} token={src.token} bind:sender={srcAddr} />
		<div class="asset-container">
			<InputAssetButton value={src} pressed={dropdown === 'src'} onClick={clickSrc} />
			<InputAmount
				chain={src.chain}
				token={src.token}
				{srcAmount}
				onChange={(amount: number) => {
					dstAmount = computeDstAmount(amount)
						? Number(computeDstAmount(amount)?.toFixed(TOKEN.get(dst.token)!.decimals))
						: 0;
					srcAmount = amount;
				}}
				readonly={waitingForBridge}
			/>
		</div>
		{#if dropdown === 'src'}
			<WalletTokenInfo isMenu={false} on:select={newSrc} available={SUPPORTED} />
		{/if}
	</div>
	<div class="flip-wrapper">
		<div class="status-content-container">
			<button
				class="down-arrow"
				onclick={() => {
					if (!$bridge) swapSrcAndDst();
				}}
				onmouseenter={() => {
					if (!$bridge) downArrowAnimation = 'up';
				}}
				onmouseleave={() => {
					if (!$bridge) downArrowAnimation = 'down';
				}}
				style:cursor={$bridge ? 'default' : 'pointer'}
				style:opacity={$bridge ? '0.3' : '1'}
			>
				{#key downArrowAnimation}
					<DownArrowIcon animation={downArrowAnimation} size="normal" />
				{/key}
			</button>
		</div>
	</div>
	<div
		class={(waitingForBridge || ($bridge && $bridge.done()) ? 'readonly' : '') +
			' content-container'}
	>
		<InputReceiver
			chain={dst.chain}
			token={dst.token}
			receiveAmount={dstAmount}
			bind:selectedAddress={dstAddr}
			readonly={waitingForBridge}
		/>
		<div class="asset-container">
			<InputAssetButton value={dst} pressed={dropdown === 'dst'} onClick={clickDst} />
			<ReceiveAmount
				{dstAmount}
				{srcAmount}
				srcToken={src.token}
				dstToken={dst.token}
				onChange={(amount) => {
					srcAmount = computeSrcAmount(amount) ? Number(computeSrcAmount(amount)?.toFixed(4)) : 0;
					dstAmount = amount;
				}}
				readonly={waitingForBridge}
			/>
		</div>
		{#if dropdown === 'dst'}
			<WalletTokenInfo isMenu={false} on:select={newDst} available={availableDst(src)} />
		{/if}
	</div>
	{#if $bridge}
		<BridgeStatus
			onClose={() => {
				if ($bridge && $bridge.done()) {
					bridge.reset();
					clock.reset();
					bridgeRequest.reset();
				}
			}}
		/>
	{/if}
	<div class="sender-receiver" style="padding: 0;">
		<div class="bridge-btn-container">
			{#if $bridge && $bridge.done()}
				<!-- Close button is inside BridgeStatus -->
			{:else if showConfirmation}
				<div class="show-confirmation-container">
					<button onclick={() => (showConfirmation = false)} class="cancel-btn"> Cancel </button>
					<button
						class="confirm-btn"
						onclick={() => {
							if (!dstAmount) return;
							start(dstAmount);
							settings = undefined;
							showConfirmation = false;
						}}
					>
						Confirm
					</button>
				</div>
			{:else if !$bridge && !$user.isConnected()}
				<button
					onclick={() => {
						$showLoginSidebar = true;
					}}
				>
					Login
				</button>
			{:else if !$bridge}
				<button
					style:opacity={isTransferAvailable() ? '1' : '0.4'}
					class:hoverable={isTransferAvailable()}
					onclick={() => {
						if (!srcAmount) {
							toasts.add(Toast.temporaryWarning('Please, provide the amount to transfer'));
							return;
						}
						if (!dstAddr) {
							toasts.add(Toast.temporaryWarning('Please, provide the destination address'));
							return;
						}
						if (dstAmount === undefined || !user) {
							toasts.add(Toast.temporaryWarning('Temporary error. Please retry.'));
							return;
						}
						if (srcAmountError) {
							toasts.add(Toast.temporaryWarning(srcAmountError));
							return;
						}
						if (typeof dstAddrError === 'string') {
							toasts.add(Toast.temporaryWarning(dstAddrError));
							return;
						}
						if (settings?.available !== undefined && dstAmount > (settings.available ?? 0)) {
							toasts.add(
								Toast.temporaryWarning(
									`Not enough liquidity on ${dst.chain}: ${settings.available} ${src.token}`
								)
							);
							return;
						}
						showConfirmation = true;
					}}
				>
					Transfer
				</button>
			{/if}
		</div>
	</div>
</div>

<style>
	.flip-wrapper {
		display: flex;
		justify-content: center;
		position: relative;
		margin: -1.25rem 0;
		z-index: 1;
	}

	.status-content-container {
		justify-content: center;
		display: flex;
		flex-direction: column;
	}

	.content-container {
		background: color-mix(in srgb, var(--c-text) 3%, var(--c-bg));
		border: var(--s-line) solid color-mix(in srgb, var(--c-text) 8%, var(--c-bg));
		border-radius: 3px;
		padding: 0.85rem 1.1rem;
		margin: 0;
		overflow: hidden;
		transition: border-color 0.2s;
	}

	.bridge-form {
		margin: 1em 0;
		display: flex;
		flex-direction: column;
		border: none;
		box-sizing: border-box;
		max-width: 26rem;
		width: 100%;
		background: var(--c-bg);
		padding: 0.35rem;
		gap: 0.3rem;
	}

	.asset-container {
		display: flex;
		align-items: center;
	}

	.readonly {
		opacity: 0.5;
	}

	.bridge-btn-container {
		margin: 0;
		text-align: center;
	}

	button {
		width: 100%;
		border: none;
	}

	button:hover {
		opacity: 0.85;
	}

	.hoverable:hover {
		opacity: 1;
	}

	.down-arrow {
		width: 2.2rem;
		height: 2.2rem;
		border-radius: 2px;
		background: var(--c-bg) !important;
		border: 2px solid color-mix(in srgb, var(--c-text) 8%, var(--c-bg)) !important;
		color: var(--c-grey) !important;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0;
		transition: color 0.15s;
	}

	.down-arrow:hover {
		color: var(--c-text) !important;
		background: var(--c-bg) !important;
	}

	.show-confirmation-container {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.5em;
		width: 100%;
	}

	.bridge-btn-container button {
		background: var(--c-text);
		color: var(--c-bg);
		border: none;
		border-radius: 3px;
		padding: 1em 1em;
		font-weight: 600;
		font-size: 0.85rem;
		font-family: 'FK Roman Standard', system-ui, serif;
		letter-spacing: 0.02em;
		transition: opacity 0.2s;
		min-height: 2.75rem;
	}

	.cancel-btn {
		background: color-mix(in srgb, var(--c-text) 6%, var(--c-bg)) !important;
		color: var(--c-text) !important;
		border: none;
		border-radius: 3px;
		padding: 0.9em 1em;
		min-width: 85px;
		width: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.confirm-btn {
		border: none;
		border-radius: 3px;
		padding: 0.9em 1em;
		min-width: 85px;
		display: flex;
		width: 50%;
		align-items: center;
		justify-content: center;
	}

	@media (max-width: 530px) {
		.content-container {
			padding: 1rem 1.1rem;
		}

		.bridge-btn-container button {
			font-size: 1.15rem;
			font-weight: 700;
		}
	}
</style>
