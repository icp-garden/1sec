import type { TransferArg, Tx } from '../../../declarations/one_sec/one_sec.did';
import type { BridgeRequest, Chain, Contracts, EvmChain, Status, Step, StepTag } from '../types';
import * as candid from '../toCandid';
import { ICP_CALL_MS, sleep } from '../utils';
import { icpAnonymous } from '$lib/user/icpUser';
import { DEV } from '$lib/env';
import { evmAnonymous } from '$lib/user/evmUser';
import { bridge } from '$lib/stores';
import { TOKEN } from '../config';
import * as fromCandid from '../fromCandid';

export class EvmToIcpFinalize implements Step {
	tag: StepTag = 'EvmToIcpFinalize';
	request: TransferArg;
	icpTx?: Tx;
	icpAmount?: number;
	transferId?: bigint;
	_status: Status;
	initialQueuePosition?: number;
	currentQueuePosition?: number;

	constructor(request: BridgeRequest) {
		this.request = candid.transferRequest(request);
		this._status = { tag: 'planned' };
	}

	label(now: Date): string {
		const s = this.status();
		switch (s.tag) {
			case 'planned':
				return 'Execute transaction';
			case 'pending':
				if (
					this.initialQueuePosition !== undefined &&
					this.currentQueuePosition !== undefined &&
					this.initialQueuePosition > 10
				) {
					const percent = Math.max(
						0,
						Math.min(
							97,
							Math.round(1 - this.currentQueuePosition / this.initialQueuePosition) * 100
						)
					);
					return `Executing transaction: ${percent}%`;
				}
				return 'Executing transaction';
			case 'ok':
				return 'Executed transaction';
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
		return this.icpAmount;
	}

	hasSentDstFunds(): boolean {
		return false;
	}

	refund(): { refund: false } {
		return { refund: false };
	}

	notify(transferId: bigint) {
		this.transferId = transferId;
	}

	async run(contracts: Contracts) {
		const transferId = this.transferId;

		if (transferId === undefined) {
			this.update({
				tag: 'err',
				error: 'missing transfer id'
			});
			return;
		}

		const oneSec = icpAnonymous().oneSec();

		const start = new Date();
		const estimatedEnd = new Date(start.getTime() + ICP_CALL_MS + ICP_CALL_MS);

		this.update({
			tag: 'pending',
			start,
			estimatedEnd: estimatedEnd
		});

		let timeout_ms = 1_000;
		while (true) {
			await sleep(timeout_ms);
			timeout_ms = Math.min(timeout_ms * 1.2, 10_000);
			const result = await oneSec.get_transfer({ id: transferId });
			switch (true) {
				case 'Ok' in result: {
					const current = result.Ok;
					const status = current.status[0];
					const candidToken = current.destination.token[0];
					if (status === undefined || candidToken == undefined) {
						break;
					}
					const token = fromCandid.token(candidToken);
					if ('PendingDestinationTx' in status) {
						if (current.queue_position.length > 0) {
							const position = Number(current.queue_position[0]) + 1;
							if (this.initialQueuePosition === undefined) {
								this.initialQueuePosition = position;
							}
							if (this.currentQueuePosition != position) {
								this.currentQueuePosition = position;
								console.log('queue: ', this.initialQueuePosition, this.currentQueuePosition);
								bridge.tick();
								timeout_ms = 1_000;
							}
						}
						break;
					} else if ('Succeeded' in status) {
						this.icpTx = current.destination.tx[0];
						this.icpAmount = fromCandid.amount(
							current.destination.amount,
							TOKEN.get(token)!.decimals
						);
						this.update({
							tag: 'ok'
						});
						return;
					} else if ('Failed' in status) {
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
