import { Principal } from '@dfinity/principal';
import type { Step, Contracts, Status, StepTag, Chain, Token } from '../types';
import { writeError } from '$lib/resultHandler';
import { ICP_CALL_MS, MS_PER_WEEK, NANOS_PER_MS } from '../utils';
import { bridge } from '$lib/stores';
import type { Tx } from '../../../declarations/one_sec/one_sec.did';
import { icpAnonymous } from '$lib/user/icpUser';

export class IcpToEvmApprove implements Step {
	tag: StepTag = 'IcpToEvmApprove';
	icpTx?: Tx;
	user: Principal;
	token: Token;
	amount: bigint;
	spender: Principal;
	_status: Status;

	constructor(user: Principal, token: Token, amount: bigint, spender: Principal) {
		this.user = user;
		this.token = token;
		this.amount = amount;
		this.spender = spender;
		this._status = {
			tag: 'planned'
		};
	}

	label(now: Date): string {
		const s = this.status();
		switch (s.tag) {
			case 'planned':
				return 'Approve transaction';
			case 'pending':
				return 'Approving transaction';
			case 'ok':
				return 'Approved transaction';
			case 'err':
				return `Failed to approve: ${s.error}`;
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

	refund(): { refund: false } {
		return { refund: false };
	}

	async run(contracts: Contracts) {
		if (!contracts.icrc2) {
			this.update({
				tag: 'err',
				error: 'wallet not connected'
			});
			return;
		}

		const start = new Date();
		const estimatedEnd = new Date(start.getTime() + ICP_CALL_MS);
		this.update({
			tag: 'pending',
			start,
			estimatedEnd: estimatedEnd
		});

		const icrc2 = contracts.icrc2;

		let allowance;

		try {
			const anonymous = icpAnonymous().ledger(this.token)!;
			allowance = await anonymous.icrc2_allowance({
				account: { owner: this.user, subaccount: [] },
				spender: { owner: this.spender, subaccount: [] }
			});
		} catch (err) {
			allowance = { allowance: BigInt(0) };
		}

		if (this.amount <= allowance.allowance) {
			this.update({
				tag: 'ok'
			});
		} else {
			try {
				const expiry = BigInt(Date.now() + MS_PER_WEEK) * BigInt(NANOS_PER_MS);
				const result = await icrc2.icrc2_approve({
					spender: { owner: this.spender, subaccount: [] },
					fee: [],
					memo: [],
					from_subaccount: [],
					created_at_time: [BigInt(Date.now()) * BigInt(NANOS_PER_MS)],
					expires_at: [expiry],
					expected_allowance: [],
					amount: this.amount
				});
				switch (true) {
					case 'Ok' in result: {
						this.icpTx = {
							Icp: {
								block_index: result['Ok'],
								ledger: this.spender
							}
						};
						this.update({
							tag: 'ok'
						});
						break;
					}
					case 'Err' in result: {
						this.update({
							tag: 'err',
							error: writeError(result.Err)
						});
						break;
					}
					default:
						this.update({
							tag: 'err',
							error: `unexpected result from ledger ${result}`
						});
						break;
				}
			} catch (err) {
				this.update({
					tag: 'err',
					error: `${err}`
				});
			}
		}
	}
	private update(status: Status) {
		this._status = status;
		bridge.tick();
	}
}
