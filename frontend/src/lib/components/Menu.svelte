<script lang="ts">
	import { goto } from '$app/navigation';
	import { createEventDispatcher, onMount } from 'svelte';
	import { SUPPORTED } from '$lib/oneSec/config';
	import { user, showingModalDialog, prices, toasts } from '$lib/stores';
	import { getWallets } from '$lib/wallet/wallet';
	import { fly } from 'svelte/transition';
	import WalletInfo from './WalletInfo.svelte';
	import WalletTokenInfo from './WalletTokenInfo.svelte';
	import Send from './Send.svelte';
	import Toast from './Toast.svelte';
	import CloseIcon from '$lib/icons/CloseIcon.svelte';
	import BackIcon from '$lib/icons/BackIcon.svelte';
	import { displayValue, truncateAddress } from '$lib/utils';
	import { Toast as ToastMessage } from '$lib/toast';
	import type { Wallet, WalletAccount } from '$lib/wallet/types';
	import { IcpUser } from '$lib/user/icpUser';
	import { EvmUser } from '$lib/user/evmUser';
	import type { EvmWallet } from '$lib/wallet/evm';

	let showWalletOptions = false;
	let isSending = false;
	let walletAccounts: WalletAccount[] = [];

	let dialog: HTMLDialogElement;
	let sheetEl: HTMLDivElement;
	let isMobile = window.innerWidth <= 530;
	let dragStartY = 0;
	let dragging = false;

	const dispatch = createEventDispatcher();

	let portfolioValue: number = 0;

	function getFilteredWallets() {
		return getWallets().filter((w) => {
			if ($user.evm && !$user.icp) return w.kind === 'icp';
			if ($user.icp && !$user.evm) return w.kind === 'evm';
			return true;
		});
	}

	function connectAccount(account: WalletAccount) {
		let userLocal = account.connect();
		switch (userLocal.wallet.kind) {
			case 'icp': {
				$user.icp = userLocal as IcpUser;
				break;
			}
			case 'evm': {
				$user.evm = userLocal as EvmUser;
				break;
			}
		}
		walletAccounts = [];
		showWalletOptions = false;
	}

	async function connectWallet(wallet: Wallet) {
		try {
			switch (wallet.kind) {
				case 'icp': {
					if ($user.icp?.wallet) {
						await $user.icp?.wallet.disconnect();
						user.reset();
					}
					break;
				}
				case 'evm': {
					localStorage.setItem('evmWalletRdns', (wallet as EvmWallet).name);
					if ($user.evm?.wallet) {
						await $user.evm?.wallet.disconnect();
						localStorage.removeItem('evmWalletRdns');
						user.reset();
					}
					break;
				}
			}

			if (wallet.name === 'Wallet Connect') {
				showWalletOptions = false;
			}
			walletAccounts = await wallet.connect();

			if (walletAccounts.length === 1) {
				connectAccount(walletAccounts[0]);
			}
		} catch (e) {
			toasts.add(
				ToastMessage.temporaryWarning(`${wallet.name} refused to connect. Please try again.`)
			);
			console.warn(e);
		}
	}

	function updatePortfolioValue() {
		let newPortfolioValue = 0;
		SUPPORTED.forEach((asset) => {
			const balance = $user.getBalance(asset.chain, asset.token)?.amount ?? 0;
			const price = $prices.get(asset.token) ?? 0;
			newPortfolioValue += balance * price;
		});
		portfolioValue = newPortfolioValue;
	}

	function onHandleTouchStart(e: TouchEvent) {
		dragStartY = e.touches[0].clientY;
		dragging = true;
		if (sheetEl) sheetEl.style.transition = 'none';
	}

	function onHandleTouchMove(e: TouchEvent) {
		if (!dragging || !sheetEl) return;
		const raw = e.touches[0].clientY - dragStartY;
		let dy: number;
		if (raw < 0) {
			// Dragging up — rubber-band resistance
			dy = -Math.sqrt(Math.abs(raw)) * 3;
		} else {
			dy = raw;
		}
		sheetEl.style.transform = `translateY(${dy}px)`;
	}

	function onHandleTouchEnd(e: TouchEvent) {
		if (!dragging || !sheetEl) return;
		dragging = false;
		const dy = e.changedTouches[0].clientY - dragStartY;
		sheetEl.style.transition = '';
		if (dy > 120) {
			sheetEl.style.transform = 'translateY(100%)';
			setTimeout(() => dialog.close(), 200);
		} else {
			sheetEl.style.transform = 'translateY(0)';
		}
	}

	function handleResize() {
		isMobile = window.innerWidth <= 530;
	}

	onMount(() => {
		dialog = document.getElementById('wallet-dialog') as HTMLDialogElement;
		dialog.showModal();

		dialog.addEventListener('click', function (event) {
			if (event.target === dialog) {
				dialog.close();
			}
		});

		window.addEventListener('resize', handleResize);

		updatePortfolioValue();
		const interval = setInterval(updatePortfolioValue, 500);

		return () => {
			clearInterval(interval);
			window.removeEventListener('resize', handleResize);
		};
	});
</script>

<dialog
	id="wallet-dialog"
	on:close={() => {
		$showingModalDialog = false;
		dispatch('close');
	}}
>
	<div
		class="slide-container"
		class:bottom-sheet={isMobile}
		bind:this={sheetEl}
		transition:fly={{ x: isMobile ? 0 : 300, y: isMobile ? 500 : 0, duration: 275 }}
	>
		{#if isMobile}
			<div
				class="sheet-handle"
				on:touchstart={onHandleTouchStart}
				on:touchmove|preventDefault={onHandleTouchMove}
				on:touchend={onHandleTouchEnd}
			>
				<div class="sheet-handle-bar"></div>
			</div>
		{/if}
		<div class="close-btn-container" class:hide-on-mobile={isMobile && !isSending}>
			{#if isSending}
				<button
					class="btn-close"
					on:click={() => {
						isSending = false;
					}}
				>
					<BackIcon size="large" />
				</button>
			{:else}
				<button
					class="btn-close"
					on:click={() => {
						dialog.close();
					}}
				>
					<CloseIcon color="black" size="normal" />
				</button>
			{/if}
		</div>
		{#if !$user.icp && !$user.evm}
			<!-- Login view - like Yusan's LoginSidebar -->
			<div class="menu-container">
				<div class="login-options">
					{#if walletAccounts.length > 0}
						<p class="login-intro">Select account</p>
						{#each walletAccounts as account, index}
							<button class="login-btn" on:click={() => connectAccount(account)}>
								{index + 1}. {truncateAddress(account.address())}
							</button>
						{/each}
					{:else}
						{#each getFilteredWallets() as wallet}
							<button class="login-btn" on:click={() => connectWallet(wallet)}>
								<img src={wallet.icon} alt={wallet.name} class="login-btn-icon" />
								{wallet.name}
							</button>
						{/each}
					{/if}
				</div>
				<p class="login-intro">Your assets will be tied to your login method.</p>
				<div class="login-bottom-spacer"></div>
				<strong class="login-disclaimer">Early development — use at your own risk.</strong>
			</div>
		{:else if !isSending}
			<div class="menu-container">
				{#if !$user.icp || !$user.evm}
					<div class="connect-section">
						<button
							class="connect-btn"
							on:click={() => {
								showWalletOptions = !showWalletOptions;
								walletAccounts = [];
							}}
						>
							{showWalletOptions ? 'Cancel' : 'Connect another wallet'}
						</button>
						{#if showWalletOptions}
							<div class="wallet-options">
								{#if walletAccounts.length > 0}
									<div class="wallet-options-header">Select account</div>
									{#each walletAccounts as account, index}
										<button class="wallet-option-btn" on:click={() => connectAccount(account)}>
											<span>{index + 1}. {truncateAddress(account.address())}</span>
										</button>
									{/each}
								{:else}
									{#each getFilteredWallets() as wallet}
										<button class="wallet-option-btn" on:click={() => connectWallet(wallet)}>
											<img src={wallet.icon} alt={wallet.name} class="wallet-option-icon" />
											<span>{wallet.name}</span>
										</button>
									{/each}
								{/if}
							</div>
						{/if}
					</div>
				{/if}
				<WalletInfo
					bind:isSending
					on:click={() => dialog.close()}
					on:closingTab={() => dialog.close()}
				/>
				<div class="total-balance-container">
					<span style="font-size: calc(0.9 * var(--small-font-size)); color: grey;"
						>Total Balance</span
					>
					<div class="animate-balance">
						<span style="--i: {0};">$</span>
						{#each displayValue(portfolioValue, 2).toString() as char, index}
							<span style="--i: {index + 1};">{char}</span>
						{/each}
					</div>
				</div>
				<WalletTokenInfo isMenu={true} available={SUPPORTED} />
			</div>
		{:else}
			<div class="menu-container">
				<Send bind:isSending />
			</div>
		{/if}
		<div class="footer-spacer"></div>
	</div>
	<Toast />
</dialog>

<style>
	::backdrop {
		background: color-mix(in srgb, var(--c-black) 30%, transparent);
	}

	a {
		display: flex;
		align-items: center;
	}

	img {
		width: var(--large-icon-size);
		height: var(--large-icon-size);
		border-radius: 50%;
	}

	.total-balance-container {
		padding: 0.5em 0.5em 0.25em;
		display: flex;
		flex-direction: column;
		align-items: end;
		margin: 0;
		border-top: var(--s-line) solid color-mix(in srgb, var(--c-text) 6%, var(--c-bg));
	}

	.connect-btn {
		animation: slide-in 300ms var(--tf-snappy);
		animation-fill-mode: forwards;
		animation-delay: 100ms;
		opacity: 0;
		transform: translateX(-5rem);
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

	dialog {
		max-width: 100dvw;
		max-height: 100dvh;
		width: 100dvw;
		height: 100dvh;
		margin: 0;
		padding: 0;
		background: none;
		border: none;
		z-index: 10;
	}

	button {
		display: flex;
		align-items: center;
		justify-content: center;
	}

	@keyframes up-and-down {
		0% {
			opacity: 0;
			transform: translateY(-1em);
		}
		100% {
			opacity: 1;
			transform: translateY(0);
		}
	}

	.animate-balance {
		display: flex;
		overflow: hidden;
	}

	.animate-balance span {
		font-size: calc(1.2 * var(--huge-font-size));
		font-weight: 800;
		letter-spacing: -0.05em;
		font-family: 'IBM Plex Mono', monospace;
		transform: translateY(-1em);
		opacity: 0;
		animation: up-and-down 200ms var(--tf-snappy);
		animation-fill-mode: forwards;
		animation-delay: calc(50ms + var(--i) * 20ms);
	}

	.slide-container {
		position: fixed;
		right: 0;
		top: 0;
		height: 100dvh;
		border-left: var(--s-line) solid var(--c-border);
		background-color: var(--c-bg);
		display: flex;
		flex-direction: column;
		overflow: auto;
		scrollbar-width: none;
	}

	.menu-container {
		display: flex;
		flex-direction: column;
		gap: 0.75em;
		padding: 1em 1.25em;
		box-sizing: border-box;
		flex: 1;
		width: 100dvw;
	}

	.close-btn-container {
		padding: 0.5em 0.75em;
		display: flex;
		justify-content: space-between;
		border-bottom: var(--s-line) solid color-mix(in srgb, var(--c-text) 6%, var(--c-bg));
	}

	.footer-spacer {
		height: 1em;
	}

	.btn-close {
		border-radius: 50%;
		padding: 0.3em;
		margin: 0;
		background: transparent !important;
		border: none !important;
		color: var(--c-text) !important;
		box-shadow: none;
	}

	.btn-close:hover {
		background-color: color-mix(in srgb, var(--c-text) 8%, transparent);
	}

	.connect-btn {
		background-color: var(--c-btn);
		color: var(--c-btn-text);
		font-size: 0.7rem;
		font-weight: 550;
		padding: 0.75em 1.2em;
		border: none;
		width: 100%;
	}

	.connect-btn:hover {
		opacity: 0.85;
	}

	.connect-section {
		display: flex;
		flex-direction: column;
		gap: 0.5em;
	}

	.wallet-options {
		display: flex;
		flex-direction: column;
		border: var(--s-line) solid color-mix(in srgb, var(--c-text) 10%, var(--c-bg));
		border-radius: 3px;
		padding: 0.25rem;
		background: var(--c-bg);
	}

	.wallet-options-header {
		font-size: 0.65rem;
		font-weight: 400;
		color: var(--c-grey);
		padding: 0.4em 0.6em 0.2em;
	}

	.wallet-option-btn {
		display: flex;
		align-items: center;
		justify-content: flex-start;
		gap: 0.5em;
		padding: 0.65em 0.6em;
		border: none !important;
		border-radius: 2px;
		background: transparent !important;
		color: var(--c-text) !important;
		font-size: 0.75rem;
		font-weight: 550;
		cursor: pointer;
		transition: background 0.1s;
		width: 100%;
		min-height: 2.5rem;
	}

	.wallet-option-btn:hover {
		background: color-mix(in srgb, var(--c-text) 5%, var(--c-bg)) !important;
	}

	.wallet-option-icon {
		width: 1.4em;
		height: 1.4em;
		border-radius: 50%;
	}

	/* Login view */
	.login-options {
		display: flex;
		flex-direction: column;
		width: 100%;
		gap: 0.35em;
	}

	.login-btn {
		display: flex;
		align-items: center;
		justify-content: flex-start;
		gap: 0.6em;
		padding: 0.85em 1em;
		border: var(--s-line) solid color-mix(in srgb, var(--c-text) 12%, var(--c-bg)) !important;
		background: var(--c-bg) !important;
		color: var(--c-text) !important;
		font-size: 0.8rem;
		font-weight: 550;
		cursor: pointer;
		transition:
			background 0.15s,
			border-color 0.15s;
		width: 100%;
		border-radius: 2px;
		min-height: 2.75rem;
	}

	.login-btn:hover {
		background: color-mix(in srgb, var(--c-text) 4%, var(--c-bg)) !important;
		border-color: color-mix(in srgb, var(--c-text) 20%, var(--c-bg)) !important;
	}

	.login-btn-icon {
		width: 1.3em;
		height: 1.3em;
		border-radius: 50%;
	}

	.login-intro {
		font-size: 0.75rem;
		font-weight: 400;
		color: var(--c-grey);
		margin-top: 0.5em;
	}

	.login-bottom-spacer {
		flex: 1;
	}

	.login-disclaimer {
		font-size: 0.6rem;
		font-weight: 600;
		color: var(--c-grey);
		text-align: center;
		padding-bottom: 1em;
	}

	/* Bottom sheet (mobile) */
	.sheet-handle {
		display: flex;
		justify-content: center;
		padding: 1em 0 0.5em;
		cursor: grab;
		touch-action: none;
	}

	.sheet-handle-bar {
		width: 2.2rem;
		height: 0.22rem;
		background: color-mix(in srgb, var(--c-text) 20%, var(--c-bg));
		border-radius: 3px;
	}

	.hide-on-mobile {
		display: none !important;
	}

	.bottom-sheet {
		position: fixed;
		top: auto !important;
		bottom: 0 !important;
		left: 0 !important;
		right: 0 !important;
		width: 100dvw !important;
		height: auto !important;
		min-height: 50dvh;
		max-height: 88dvh;
		border-left: none !important;
		border-top: none;
		border-radius: 3px 3px 0 0;
		transition: transform 0.25s cubic-bezier(0.3, 0.7, 0, 1);
		box-shadow: 0 6rem 0 0 var(--c-bg);
	}

	.bottom-sheet .menu-container {
		max-width: 100%;
		width: 100%;
		padding-bottom: env(safe-area-inset-bottom, 1em);
	}

	.bottom-sheet .footer-spacer {
		height: env(safe-area-inset-bottom, 0.5em);
	}

	@media (max-width: 530px) {
		.slide-container {
			width: 100dvw;
		}
	}

	@media (min-width: 531px) {
		.slide-container {
			width: 380px;
		}

		.menu-container {
			max-width: 380px;
		}
	}
</style>
