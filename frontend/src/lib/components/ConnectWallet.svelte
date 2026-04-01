<script lang="ts">
	import { toasts, user, showingModalDialog } from '$lib/stores';
	import { Toast as ToastMessage } from '$lib/toast';
	import { fade } from 'svelte/transition';
	import { createEventDispatcher, onMount } from 'svelte';
	import CloseIcon from '$lib/icons/CloseIcon.svelte';
	import Toast from './Toast.svelte';
	import type { Wallet, WalletAccount } from '$lib/wallet/types';
	import { IcpUser } from '$lib/user/icpUser';
	import { EvmUser } from '$lib/user/evmUser';
	import { truncateAddress, isMobile } from '$lib/utils';
	import type { EvmWallet } from '$lib/wallet/evm';

	export let wallets: Wallet[];
	let accounts: WalletAccount[];

	let connecting: boolean = false;
	let dialog: HTMLDialogElement;

	const dispatch = createEventDispatcher();

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
		accounts = [];
		dialog.close();
	}

	async function connectWallet(wallet: Wallet): Promise<WalletAccount[]> {
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
				dialog.close();
			}
			accounts = await wallet.connect();

			if (accounts.length === 1) {
				connectAccount(accounts[0]);
			}
			return accounts;
		} catch (e) {
			toasts.add(
				ToastMessage.temporaryWarning(`${wallet.name} refused to connect. Please try again.`)
			);
			console.warn(e);
			return [];
		}
	}

	onMount(() => {
		dialog = document.getElementById('connect-dialog') as HTMLDialogElement;
		dialog.addEventListener('cancel', (event) => {
			if (connecting) {
				event.preventDefault();
			}
		});
		dialog.showModal();

		dialog.addEventListener('click', function (event) {
			if (event.target === dialog) {
				dialog.close();
			}
		});
	});
</script>

<dialog
	id="connect-dialog"
	on:close={() => {
		$showingModalDialog = false;
		dispatch('close');
	}}
>
	<div class="content-container">
		<div class="wallets-container" in:fade={{ duration: 150 }}>
			{#if accounts && accounts.length > 0}
				<div class="header-container">
					<h2>Select account</h2>
					<button
						on:click={() => {
							dialog.close();
						}}
						class="close-btn"
					>
						<CloseIcon color="black" size="normal" />
					</button>
				</div>
				<div class="selection-container">
					{#each accounts as account, index}
						<button class="login-btn" on:click={() => connectAccount(account)}>
							<span>{index + 1}. {truncateAddress(account.address())}</span>
						</button>
					{/each}
				</div>
			{:else}
				<div class="header-container">
					<h2>Connect wallet</h2>
					{#if !connecting}
						<button
							on:click={() => {
								dialog.close();
							}}
							class="close-btn"
						>
							<CloseIcon color="black" size="normal" />
						</button>
					{/if}
				</div>
				<div class="selection-container">
					{#each wallets as wallet}
						<button
							class="login-btn"
							on:click={async () => (accounts = await connectWallet(wallet))}
						>
							<img src={wallet.icon} width="40em" height="40em" alt={wallet.name} />
							<span>{wallet.name}</span>
						</button>
					{/each}
					{#if wallets.length === 0}
						<p style="font-size: 1em;">No wallets detected for this chain</p>
					{/if}
				</div>
			{/if}
		</div>
	</div>
	<Toast />
</dialog>

<style>
	/* === Base Styles === */
	::backdrop {
		background: color-mix(in srgb, var(--c-black) 50%, transparent);
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
		z-index: 15;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	img {
		border-radius: 50%;
	}

	.content-container {
		border: var(--s-line) solid var(--c-border);
		box-sizing: border-box;
		padding: 1.5em;
		background-color: var(--c-bg);
	}

	h2 {
		font-size: var(--normal-font-size);
		font-weight: 600;
	}

	.wallets-container {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		max-width: 40em;
	}

	.header-container {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 1em;
	}

	.selection-container {
		display: flex;
		flex-direction: column;
		gap: 0.4em;
	}

	.login-btn {
		padding: 0.6em 0.8em;
		gap: 1em;
		display: flex;
		align-items: center;
		background: transparent;
		border: var(--s-line) solid var(--c-border);
		color: var(--c-text);
	}

	.login-btn:hover {
		transition: all 0.15s;
		background: color-mix(in srgb, var(--c-text) 5%, transparent);
	}

	.close-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 50%;
		padding: 0.3em;
		background: transparent;
		border: none;
		color: var(--c-text);
	}

	.close-btn:hover {
		background-color: color-mix(in srgb, var(--c-text) 8%, transparent);
	}
</style>
