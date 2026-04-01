import type { EvmUser } from '$lib/user/evmUser';
import { IcpUser } from '$lib/user/icpUser';
import * as evm from './evm';
import * as ii from './ii';
import * as nfid from './nfid';
import * as plug from './plug';
import * as wc from './walletconnect';
import * as coinbase from './coinbasewallet';
import type { Wallet } from './types';

declare global {
	interface Window {
		ic: any; // Or use a more specific type if available
	}
}

export function getWallets(): Wallet[] {
	let result = [
		ii.getWallet(),
		// nfid.getWallet(),
		wc.getWallet(),
		coinbase.getWallet(),
		...evm.getWallets()
	];
	if (window.ic && window.ic.plug) {
		if (plug.getWallet()) {
			result.push(plug.getWallet()!);
		}
	}
	return result;
}

export async function tryReconnectInternetIdentity(): Promise<IcpUser | undefined> {
	return ii.tryReconnect();
}

export async function tryReconnectEvm(): Promise<EvmUser | undefined> {
	return evm.tryReconnect();
}
