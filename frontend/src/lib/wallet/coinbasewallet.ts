import { createCoinbaseWalletSDK } from '@coinbase/wallet-sdk';
import type { Wallet, WalletAccount, WalletKind } from './types';
import { ethers, type JsonRpcSigner } from 'ethers';
import { EvmUser } from '$lib/user/evmUser';

const sdk = createCoinbaseWalletSDK({
	appName: 'OneSec',
	appLogoUrl: ''
});

export class CoinBaseWallet implements Wallet {
	name = 'Coinbase Wallet';
	icon = '/icons/wallet/coinbase.svg';
	kind: WalletKind = 'evm';

	async connect(): Promise<WalletAccount[]> {
		const provider = sdk.getProvider();
		// Required by CoinbaseWallet to connect.
		const _ = await provider.request({
			method: 'eth_requestAccounts'
		});
		const ethersProvider = new ethers.BrowserProvider(provider);

		const signer = await ethersProvider.getSigner();
		return [new CoinBaseWalletAccount(this, signer)];
	}

	async disconnect(): Promise<void> {
		const provider = sdk.getProvider();
		await provider.disconnect();
	}

	async isExpired(): Promise<boolean> {
		return false;
	}
}

export class CoinBaseWalletAccount implements WalletAccount {
	wallet: CoinBaseWallet;
	signer: JsonRpcSigner;

	constructor(wallet: CoinBaseWallet, signer: JsonRpcSigner) {
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

let wallet = new CoinBaseWallet();
export function getWallet(): Wallet {
	return wallet;
}
