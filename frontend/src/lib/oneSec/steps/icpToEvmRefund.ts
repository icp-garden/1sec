import type { Tx } from '../../../declarations/one_sec/one_sec.did';
import type { Chain, Contracts, Status, Step, StepTag } from '../types';
import { ICP_CALL_MS, sleep } from '../utils';
import { icpAnonymous } from '$lib/user/icpUser';
import { bridge } from '$lib/stores';
import * as fromCandid from '../fromCandid';
import { TOKEN } from '../config';

export class IcpToEvmRefund implements Step {
	tag: StepTag = 'IcpToEvmRefund';
	refundTx?: Tx;
	refundAmount?: number;
	transferId: bigint;
	_status: Status;
	initialQueuePosition?: number;
	currentQueuePosition?: number;

	constructor(transferId: bigint) {
		this.transferId = transferId;
		this._status = { tag: 'planned' };
	}

	label(now: Date): string {
		const s = this.status();
		switch (s.tag) {
			case 'planned':
				return 'Execute refund';
			case 'pending': {
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
					return `Executing refund: ${percent}%`;
				}
				return 'Executing refund';
			}
			case 'ok':
				return 'Executed refund';
			case 'err':
				return `Failed refund: ${s.error}`;
		}
	}

	status(): Status {
		return this._status;
	}

	chain(): Chain {
		return 'ICP';
	}

	tx(): Tx | undefined {
		return this.refundTx;
	}

	dstAmount(): number | undefined {
		return this.refundAmount;
	}

	hasSentDstFunds(): boolean {
		return false;
	}

	refund(): { refund: false } {
		return {
			refund: false
		};
	}

	async run(contracts: Contracts) {
		const start = new Date();
		const estimatedEnd = new Date(start.getTime() + ICP_CALL_MS + ICP_CALL_MS);
		this.update({
			tag: 'pending',
			start,
			estimatedEnd: estimatedEnd
		});

		const anonymous = icpAnonymous().oneSec();

		const transferId = this.transferId;

		let timeout_ms = 1_000;

		while (true) {
			await sleep(timeout_ms);
			timeout_ms = Math.min(timeout_ms * 1.2, 10_000);
			const result = await anonymous.get_transfer({ id: transferId });
			switch (true) {
				case 'Ok' in result: {
					const current = result.Ok;
					const status = current.status[0];
					const candidToken = current.source.token[0];
					if (status === undefined || candidToken == undefined) {
						break;
					}
					if ('Refunded' in status) {
						this.refundTx = status.Refunded;
						const token = fromCandid.token(candidToken);
						this.refundAmount = fromCandid.amount(
							current.source.amount,
							TOKEN.get(token)!.decimals
						);
						this.update({
							tag: 'ok'
						});
						return;
					} else if ('PendingRefundTx' in status) {
						if (current.queue_position.length > 0) {
							const position = Number(current.queue_position[0]) + 1;
							if (this.initialQueuePosition === undefined) {
								this.initialQueuePosition = position;
							}
							if (this.currentQueuePosition != position) {
								this.currentQueuePosition = position;
								bridge.tick();
								timeout_ms = 1_000;
							}
						}
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
