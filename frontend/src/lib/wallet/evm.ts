import type { Wallet, WalletAccount, WalletKind } from './types';
import { ethers } from 'ethers';
import type { Eip1193Provider } from 'ethers';
import { EvmUser } from '$lib/user/evmUser';
import type { JsonRpcSigner } from 'ethers';
import { get } from 'svelte/store';
import { user } from '$lib/stores';

interface Eip6963ProviderInfo {
	uuid: string;
	name: string;
	icon: string;
	rdns: string;
}

interface EIP6963ProviderDetail {
	info: Eip6963ProviderInfo;
	provider: Eip1193Provider;
}

interface EIP6963AnnounceProviderEvent extends CustomEvent {
	type: 'eip6963:announceProvider';
	detail: EIP6963ProviderDetail;
}

export class EvmWallet implements Wallet {
	name: string;
	icon: string;
	kind: WalletKind = 'evm';
	rdns: string;
	provider: Eip1193Provider;

	constructor(info: Eip6963ProviderInfo, provider: Eip1193Provider) {
		this.name = info.name;
		this.icon = info.icon;
		this.provider = provider;
		this.rdns = info.rdns;
	}

	async connect(): Promise<WalletAccount[]> {
		const accountProvider = new ethers.BrowserProvider(this.provider, 'any');
		const signer = await accountProvider.getSigner();
		return [new EvmWalletAccount(this, signer)];
	}

	async disconnect(): Promise<void> {
		// No op.
	}

	async isExpired(): Promise<boolean> {
		return false;
	}
}

export class EvmWalletAccount implements WalletAccount {
	wallet: EvmWallet;
	signer: JsonRpcSigner;

	constructor(wallet: EvmWallet, signer: JsonRpcSigner) {
		this.wallet = wallet;
		this.signer = signer;
	}

	address(): string {
		return this.signer.address;
	}

	connect(): EvmUser {
		return new EvmUser(this.signer.address, this.signer, this.wallet);
	}
}

export let walletDetails: EIP6963ProviderDetail[] = [];
window.addEventListener('eip6963:announceProvider', (event) => {
	const detail = (event as EIP6963AnnounceProviderEvent).detail;
	walletDetails.push(detail);
});

window.dispatchEvent(new Event('eip6963:requestProvider'));

let wallets: Map<string, EvmWallet> = new Map();
walletDetails.forEach((detail) => {
	(detail.provider as any).on('accountsChanged', async (accounts: string[]) => {
		let currentUser = get(user);
		if (!currentUser.evm) return;
		const wallet = await currentUser.evm.wallet.connect();
		currentUser.evm = wallet[0].connect() as EvmUser;
		user.set(currentUser);
	});
	if (detail.info.name !== 'Plug wallet') {
		wallets.set(detail.info.name, new EvmWallet(detail.info, detail.provider));
	}
});

export function getWallets(): Wallet[] {
	return Array.from(wallets.values());
}

export async function tryReconnect(): Promise<EvmUser | undefined> {
	const maybeRdns = localStorage.getItem('evmWalletRdns');
	if (!maybeRdns) return;

	const wallet = wallets.get(maybeRdns);
	if (wallet) {
		const accounts = await wallet.connect();
		return accounts[0].connect() as EvmUser;
	}
}
