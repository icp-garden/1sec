import type { BridgeRequest, Chain, Contracts, Status, Step, StepTag, Token } from '../types';
import { ICP_CALL_MS } from '../utils';
import type { Tx } from '../../../declarations/one_sec/one_sec.did';
import { MAINNET_ICP_TOKENS } from '../config';
import type * as candid from '../../../declarations/ck_unwrap/ck_unwrap.did';
import { bridge } from '$lib/stores';

export class ckToOneSec implements Step {
	tag: StepTag = 'ckToOneSec';
	request: BridgeRequest;
	tokenName: Token;
	amount: bigint;
	transferId?: bigint;
	icpTx?: Tx;
	refunding: boolean;
	_status: Status;

	constructor(request: BridgeRequest, amount: bigint, tokenName: Token) {
		this.request = request;
		this.tokenName = tokenName;
		this.amount = amount;
		this.refunding = false;
		this._status = { tag: 'planned' };
	}

	label(now: Date): string {
		const s = this.status();
		switch (s.tag) {
			case 'planned':
				return 'Execute transaction';
			case 'pending': {
				return `Transferring ck tokens to the ckUnwrapper`;
			}
			case 'ok':
				return `Transferred ck tokens to the ckUnwrapper`;
			case 'err':
				return `Failed transaction: ${s.error}`;
		}
	}

	status(): Status {
		return this._status;
	}

	chain(): Chain {
		return 'ICP';
	}

	tx(): Tx | undefined {
		return this.icpTx;
	}

	dstAmount(): number | undefined {
		return undefined;
	}

	hasSentDstFunds(): boolean {
		return false;
	}

	refund(): { refund: true; transferId: bigint } | { refund: false } {
		if (this.refunding) {
			return {
				refund: true,
				transferId: this.transferId!
			};
		} else {
			return {
				refund: false
			};
		}
	}

	async run(contracts: Contracts) {
		if (!contracts.ckUnwrap) {
			this.update({
				tag: 'err',
				error: 'No connection to canister'
			});
			return;
		}

		const ckUnwrap = contracts.ckUnwrap;

		const start = new Date();
		const estimatedEnd = new Date(start.getTime() + ICP_CALL_MS + ICP_CALL_MS);
		this.update({
			tag: 'pending',
			start,
			estimatedEnd: estimatedEnd
		});

		// source: {
		//         account: { Icp: r.icpAccount },
		//         chain: { ICP: null },
		//         token: token(r.icpToken),
		//         amount: r.icpAmount,
		//         tx: []
		//     },
		let ledger_canister_id;

		if (this.request.icpToken == 'ckUSDC') {
			ledger_canister_id = MAINNET_ICP_TOKENS.get('ckUSDC')!.ledger;
		} else if (this.request.icpToken == 'ckUSDT') {
			ledger_canister_id = MAINNET_ICP_TOKENS.get('ckUSDT')!.ledger;
		}

		if (ledger_canister_id == undefined) {
			this.update({
				tag: 'err',
				error: `only ckUSDC and ckUSDT supported`
			});
			return;
		}

		try {
			const result = await ckUnwrap.unwrap_ck_to_onesec({
				from: this.request.icpAccount,
				amount_e6s: this.request.icpAmount,
				ledger_canister_id: ledger_canister_id.toString()
			} as candid.UnwrapArgs);
			console.log(result);

			switch (true) {
				case 'Ok' in result:
					this.transferId = result.Ok;
					this.update({
						tag: 'ok'
					});
					break;
				case 'Err' in result:
					this.update({
						tag: 'err',
						error: result.Err
					});
					return;
			}
		} catch (err) {
			this.update({
				tag: 'err',
				error: `${err}`
			});
			return;
		}
	}

	private update(status: Status) {
		this._status = status;
		bridge.tick();
	}
}
