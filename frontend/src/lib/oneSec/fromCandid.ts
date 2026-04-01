import type * as candid from '../../declarations/one_sec/one_sec.did';
import type { Chain, Token } from './types';
import { Asset } from './types';

export function token(token: candid.Token): Token {
	switch (true) {
		case 'ICP' in token:
			return 'ICP';
		case 'USDC' in token:
			return 'USDC';
		case 'USDT' in token:
			return 'USDT';
		case 'cbBTC' in token:
			return 'cbBTC';
		case 'BOB' in token:
			return 'BOB';
		case 'ckBTC' in token:
			return 'ckBTC';
		case 'GLDT' in token:
			return 'GLDT';
		case 'CHAT' in token:
			return 'CHAT';
		default: {
			// This ensures that all variants are covered above.
			const _exhaustiveCheck: never = token;
			throw `unexpected candid token: ${token}`;
		}
	}
}

export function chain(chain: candid.Chain): Chain {
	switch (true) {
		case 'ICP' in chain:
			return 'ICP';
		case 'Base' in chain:
			return 'Base';
		case 'Arbitrum' in chain:
			return 'Arbitrum';
		case 'Ethereum' in chain:
			return 'Ethereum';
		default: {
			// This ensures that all variants are covered above.
			const _exhaustiveCheck: never = chain;
			throw `unexpected candid token: ${chain}`;
		}
	}
}

export function asset(c: candid.Chain, t: candid.Token): Asset {
	return new Asset(chain(c), token(t));
}

export function amount(amount: bigint, decimals: number): number {
	const x10 = BigInt(10);
	while (decimals > 8) {
		amount /= x10;
		--decimals;
	}
	// Since the amount was reduces it fits into 53-bits.
	return Number(amount) / 10 ** decimals;
}
