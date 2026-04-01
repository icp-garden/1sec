import WalletConnectProvider from '@walletconnect/ethereum-provider';
import { ethers } from 'ethers';
import type { Wallet, WalletAccount, WalletKind } from './types';
import { EvmUser } from '$lib/user/evmUser';
import type { JsonRpcSigner } from 'ethers';

export class WalletConnect implements Wallet {
	name = 'Wallet Connect';
	icon = '/icons/wallet/wallet-connect.svg';
	kind: WalletKind = 'evm';

	async connect(): Promise<WalletAccount[]> {
		const wcProvider = await WalletConnectProvider.init({
			projectId: 'efdd1b2983098c356412f02fc4580cb1',
			showQrModal: true,
			chains: [1],
			optionalChains: [1, 8453, 42161],
			rpcMap: {
				1: 'https://eth.llamarpc.com',
				8453: 'https://base.llamarpc.com',
				42161: 'https://arbitrum.drpc.org'
			}
		});
		await wcProvider.connect();
		const ethersProvider = new ethers.BrowserProvider(wcProvider);

		const signer = await ethersProvider.getSigner();
		return [new WalletConnectAccount(this, signer)];
	}

	async disconnect(): Promise<void> {
		// No op.
	}

	async isExpired(): Promise<boolean> {
		return false;
	}
}

export class WalletConnectAccount implements WalletAccount {
	wallet: WalletConnect;
	signer: JsonRpcSigner;

	constructor(wallet: WalletConnect, signer: JsonRpcSigner) {
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

let wallet = new WalletConnect();
export function getWallet(): Wallet {
	return wallet;
}
