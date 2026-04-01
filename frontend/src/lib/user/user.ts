import type { EvmUser } from './evmUser';
import type { IcpUser } from './icpUser';
import type { Chain, Token } from '$lib/oneSec/types';
import { type Balance } from '$lib/types';
import { SUPPORTED } from '$lib/oneSec/config';

export class User {
	icp: IcpUser | undefined;
	evm: EvmUser | undefined;

	constructor() {}

	getBalance(chain: Chain, token: Token): Balance | undefined {
		if (chain === 'ICP') {
			return this.icp?.getBalance(token);
		} else {
			return this.evm?.getBalance(chain, token);
		}
	}

	tokens(chain?: Chain): Token[] {
		return SUPPORTED.filter((asset) => asset.chain == chain).map((asset) => asset.token);
	}

	isConnected(): boolean {
		return !(this.icp == undefined && this.evm == undefined);
	}
}
