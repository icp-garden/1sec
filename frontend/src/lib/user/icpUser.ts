import { DEV } from '$lib/env';
import type { Token } from '$lib/oneSec/types';
import { Actor, HttpAgent, type Agent } from '@dfinity/agent';
import { Principal } from '@dfinity/principal';
import type { _SERVICE as ICRC2 } from '../../declarations/icrc_ledger/icrc_ledger.did';
import type { _SERVICE as ICRC1 } from '../../declarations/icp_ledger/icp_ledger.did';
import type { _SERVICE as OneSec } from '../../declarations/one_sec/one_sec.did';
import type { _SERVICE as CkUnwrap } from '../../declarations/ck_unwrap/ck_unwrap.did';
import { idlFactory as idlOneSec } from '../../declarations/one_sec';
import { idlFactory as idlCkUnwrap } from '../../declarations/ck_unwrap';
import { idlFactory as idlICRC2 } from '../../declarations/icrc_ledger';
import { idlFactory as idlICRC1 } from '../../declarations/icp_ledger';
import { CONFIG, TOKEN } from '$lib/oneSec/config';
import type { Wallet } from '$lib/wallet/types';
import { anonymousAgent, anonymousWallet } from '$lib/wallet/ii';
import { type Balance } from '$lib/types';
import * as fromCandid from '$lib/oneSec/fromCandid';
import { user } from '$lib/stores';
import { AccountIdentifier } from '@dfinity/ledger-icp';

export class IcpUser {
	principal: Principal;
	accountId: AccountIdentifier;
	agent: Agent;
	wallet: Wallet;
	_ledger: Map<Token, ICRC2 | ICRC1>;
	_oneSec?: OneSec;
	_ckUnwrap?: CkUnwrap;
	_balance: Map<Token, Balance>;

	constructor(principal: Principal, accountId: AccountIdentifier, agent: Agent, wallet: Wallet) {
		if (DEV) {
			agent.fetchRootKey().catch((err) => {
				console.warn(
					'Unable to fetch root key. Check to ensure that your local replica is running'
				);
				console.error(err);
			});
		}
		this.principal = principal;
		this.accountId = accountId;
		this.agent = agent;
		this.wallet = wallet;
		this._ledger = new Map();
		this._balance = new Map();
	}

	ledger(token: Token): ICRC2 | ICRC1 | undefined {
		if (this._ledger.has(token)) {
			return this._ledger.get(token);
		}
		const config = CONFIG.icp.token.get(token);
		if (!config) {
			console.error(`no ICP config for token ${token}`);
			return undefined;
		}

		let ledger: ICRC1 | ICRC2;
		if (token === 'ICP') {
			ledger = Actor.createActor(idlICRC1, {
				agent: this.agent,
				canisterId: config.ledger
			});
			this._ledger.set(token, ledger);
		} else {
			ledger = Actor.createActor(idlICRC2, {
				agent: this.agent,
				canisterId: config.ledger
			});
			this._ledger.set(token, ledger);
		}
		return ledger;
	}

	oneSec(): OneSec {
		if (this._oneSec) {
			return this._oneSec;
		}
		const config = CONFIG.icp;
		const oneSec: OneSec = Actor.createActor(idlOneSec, {
			agent: this.agent,
			canisterId: config.oneSec
		});
		this._oneSec = oneSec;
		return oneSec;
	}

	ckUnwrap(): CkUnwrap {
		if (this._ckUnwrap) {
			return this._ckUnwrap;
		}
		const config = CONFIG.icp;
		const ckUnwrap: CkUnwrap = Actor.createActor(idlCkUnwrap, {
			agent: this.agent,
			canisterId: config.ckUnwrap
		});
		this._ckUnwrap = ckUnwrap;
		return ckUnwrap;
	}

	tokens(): Token[] {
		return Array.from(CONFIG.icp.token.keys());
	}

	getBalance(token: Token): Balance | undefined {
		return this._balance.get(token);
	}

	async fetchBalance(token: Token) {
		const config = CONFIG.icp.token.get(token);

		if (!config) {
			console.error(`no ICP config for token ${token}`);
			return;
		}
		const ledger: ICRC2 = Actor.createActor(idlICRC2, {
			agent: anonymousAgent(),
			canisterId: config.ledger
		});

		const result = await ledger.icrc1_balance_of({ owner: this.principal, subaccount: [] });
		const amount = fromCandid.amount(result, TOKEN.get(token)!.decimals);
		this._balance.set(token, { amount, lastUpdated: new Date() });
		user.tick();
	}
}

let _icpAnonymous: IcpUser | undefined = undefined;
export function icpAnonymous(): IcpUser {
	if (_icpAnonymous) {
		return _icpAnonymous;
	}
	let user = new IcpUser(
		Principal.anonymous(),
		AccountIdentifier.fromPrincipal({ principal: Principal.anonymous() }),
		anonymousAgent(),
		anonymousWallet()
	);
	_icpAnonymous = user;
	return user;
}
