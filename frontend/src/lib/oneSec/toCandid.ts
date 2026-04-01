import type * as candid from '../../declarations/one_sec/one_sec.did';
import type { BridgeRequest, EvmChain, Token } from './types';

export function token(token: Token): candid.Token {
	switch (token) {
		case 'ICP':
			return { ICP: null };
		case 'USDC':
			return { USDC: null };
		case 'USDT':
			return { USDT: null };
		case 'cbBTC':
			return { cbBTC: null };
		case 'ckBTC':
			return { ckBTC: null };
		case 'BOB':
			return { BOB: null };
		case 'GLDT':
			return { GLDT: null };
		case 'CHAT':
			return { CHAT: null };
	}
}

export function chain(chain: EvmChain): candid.Chain {
	switch (chain) {
		case 'Base':
			return { Base: null };
		case 'Arbitrum':
			return { Arbitrum: null };
		case 'Ethereum':
			return { Ethereum: null };
	}
}

export function transferRequest(r: BridgeRequest): candid.TransferArg {
	switch (r.direction) {
		case 'IcpToEvm': {
			return {
				source: {
					account: { Icp: r.icpAccount },
					chain: { ICP: null },
					token: token(r.icpToken),
					amount: r.icpAmount,
					tx: []
				},
				destination: {
					account: { Evm: { address: r.evmAccount } },
					chain: chain(r.evmChain),
					token: token(r.evmToken),
					amount: [r.evmAmount]
				}
			};
		}
		case 'EvmToIcp': {
			return {
				source: {
					account: { Evm: { address: r.evmAccount } },
					chain: chain(r.evmChain),
					token: token(r.evmToken),
					amount: r.evmAmount,
					tx: []
				},
				destination: {
					account: { Icp: r.icpAccount },
					chain: { ICP: null },
					token: token(r.icpToken),
					amount: [r.icpAmount]
				}
			};
		}
	}
}

export function amount(amount: number, decimals: number): bigint {
	const [integerPart, fractionalPart = ''] = amount.toFixed(decimals).split('.');
	const paddedFractionalPart = fractionalPart.padEnd(decimals, '0').slice(0, decimals);
	const combined = `${integerPart}${paddedFractionalPart}`;
	return BigInt(combined);
}
