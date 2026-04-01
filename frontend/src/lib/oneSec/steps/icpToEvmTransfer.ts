import type { Tx } from '../../../declarations/one_sec/one_sec.did';
import { IcpToEvmWaitForTx } from './icpToEvmWaitForTx';
import { IcpToEvmFinalize } from './icpToEvmFinalize';
import type { BridgeRequest, Chain, Contracts, Status, Step, StepTag, Token } from '../types';
import * as candid from '../toCandid';
import { ICP_CALL_MS, sleep } from '../utils';
import { icpAnonymous } from '$lib/user/icpUser';
import { displayValue } from '$lib/utils';
import { tokenToDecimals } from '../config';
import { bridge, bridgeSettings } from '$lib/stores';

export class IcpToEvmTransfer implements Step {
	tag: StepTag = 'IcpToEvmTransfer';
	request: BridgeRequest;
	tokenName: Token;
	amount: bigint;
	transferId?: bigint;
	icpTx?: Tx;
	waitForTx: IcpToEvmWaitForTx;
	finalize: IcpToEvmFinalize;
	refunding: boolean;
	_status: Status;

	constructor(
		request: BridgeRequest,
		waitForTx: IcpToEvmWaitForTx,
		finalize: IcpToEvmFinalize,
		amount: bigint,
		tokenName: Token
	) {
		this.request = request;
		this.waitForTx = waitForTx;
		this.finalize = finalize;
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
				return `Transferring ${Number(this.amount) / Math.pow(10, tokenToDecimals(this.tokenName))} ${this.tokenName} to OneSec`;
			}
			case 'ok':
				return `Transferred ${Number(this.amount) / Math.pow(10, tokenToDecimals(this.tokenName))} ${this.tokenName} to OneSec`;
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
		if (!contracts.oneSec) {
			this.update({
				tag: 'err',
				error: 'No connection to canister'
			});
			return;
		}

		const oneSec = contracts.oneSec;

		const start = new Date();
		const estimatedEnd = new Date(start.getTime() + ICP_CALL_MS + ICP_CALL_MS);
		this.update({
			tag: 'pending',
			start,
			estimatedEnd: estimatedEnd
		});

		try {
			const result = await oneSec.transfer(candid.transferRequest(this.request));

			switch (true) {
				case 'Accepted' in result:
					this.transferId = result.Accepted.id;
					this.waitForTx.notify(this.transferId);
					this.finalize.notify(this.transferId);
					break;
				case 'Failed' in result:
					this.update({
						tag: 'err',
						error: result.Failed.error
					});
					return;
				case 'Fetching' in result:
					this.update({
						tag: 'err',
						error: `unexpected result: ${result}`
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

		const anonymous = icpAnonymous().oneSec();

		const transferId = this.transferId as bigint;

		let timeout_ms = 1_000;

		while (true) {
			await sleep(timeout_ms);
			timeout_ms = Math.min(timeout_ms * 1.2, 10_000);
			const result = await anonymous.get_transfer({ id: transferId });
			switch (true) {
				case 'Ok' in result: {
					const current = result.Ok;
					const status = current.status[0];
					if (status === undefined) {
						break;
					}
					if ('PendingSourceTx' in status) {
						// Nothing to do, waiting.
						break;
					} else if ('Refunded' in status || 'PendingRefundTx' in status) {
						this.refunding = true;
						this.icpTx = current.source.tx[0];
						this.update({
							tag: 'ok'
						});
						return;
					} else if ('Succeeded' in status || 'PendingDestinationTx' in status) {
						this.icpTx = current.source.tx[0];
						this.update({
							tag: 'ok'
						});
						return;
					} else if ('Failed' in status) {
						if (status.Failed.error.includes('fee')) {
							bridgeSettings.update((b) => {
								b.update();
								return b;
							});
						}
						this.update({
							tag: 'err',
							error: status.Failed.error
						});
						return;
					} else {
						this.update({
							tag: 'err',
							error: `unexpected status: ${status}`
						});
						return;
					}
				}
				case 'Err' in result: {
					console.log(`get_transfer failed: ${result.Err}`);
					break;
				}
				default:
					console.log(`unexpected result of get_transfer: ${result}`);
					break;
			}
		}
	}

	private update(status: Status) {
		this._status = status;
		bridge.tick();
	}
}
