import type { Step, Contracts, Status, StepTag, EvmChain, Chain } from '../types';
import type { TraceEvent, Tx } from '../../../declarations/one_sec/one_sec.did';
import { ICP_CALL_MS, sleep } from '../utils';
import { icpAnonymous } from '$lib/user/icpUser';
import { DEV } from '$lib/env';
import { evmAnonymous } from '$lib/user/evmUser';
import { bridge } from '$lib/stores';
import { TOKEN } from '../config';
import * as fromCandid from '../fromCandid';

type TxStatus = 'unknown' | 'signed' | 'sent' | 'executed';

function order(ts: TxStatus) {
	switch (ts) {
		case 'unknown':
			return 0;
		case 'signed':
			return 1;
		case 'sent':
			return 2;
		case 'executed':
			return 3;
	}
}

function traceEventToTxStatus(event: TraceEvent): TxStatus {
	switch (true) {
		case 'SignTx' in event:
			return 'signed';
		case 'SendTx' in event:
			return 'sent';
		case 'ConfirmTx' in event:
			return 'executed';
		case 'PendingConfirmTx' in event:
			return 'executed';
		case 'FetchTx' in event:
			return 'unknown';
	}
	return 'unknown';
}

export class IcpToEvmWaitForTx implements Step {
	tag: StepTag = 'IcpToEvmWaitForTx';
	evmChain: EvmChain;
	evmTx?: Tx;
	evmAmount?: number;
	txStatus: TxStatus;
	transferId?: bigint;
	_status: Status;

	initialQueuePosition?: number;
	currentQueuePosition?: number;

	constructor(chain: EvmChain) {
		this.evmChain = chain;
		this.txStatus = 'unknown';
		this._status = { tag: 'planned' };
	}

	label(now: Date): string {
		const s = this.status();
		switch (s.tag) {
			case 'planned':
				return `Execute ${this.evmChain} transaction`;
			case 'pending': {
				switch (this.txStatus) {
					case 'unknown':
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
							return `Signing transaction: ${percent}%`;
						}
						return `Signing transaction`;
					case 'signed':
						return `Sending transaction`;
					case 'sent':
						return `Executing transaction`;
					case 'executed':
						return `Executed transaction`;
				}
			}
			case 'ok':
				return `Executed transaction`;
			case 'err':
				return `Failed transaction: ${s.error}`;
		}
	}

	status(): Status {
		return this._status;
	}

	chain(): Chain {
		return this.evmChain;
	}

	tx(): Tx | undefined {
		return this.evmTx;
	}

	dstAmount(): number | undefined {
		return this.evmAmount;
	}

	hasSentDstFunds(): boolean {
		if (!this.evmTx) {
			return false;
		}
		return this.txStatus === 'executed';
	}

	refund(): { refund: false } {
		return { refund: false };
	}

	notify(transferId?: bigint) {
		this.transferId = transferId;
	}

	async run(contracts: Contracts) {
		if (this.transferId == null) {
			this.update({
				tag: 'err',
				error: 'missing transfer id'
			});
			return;
		}

		const transferId = this.transferId;

		const anonymous = icpAnonymous().oneSec();

		const start = new Date();
		const estimatedEnd = new Date(start.getTime() + ICP_CALL_MS + ICP_CALL_MS);
		this.update({
			tag: 'pending',
			start,
			estimatedEnd: estimatedEnd
		});

		let timeout_ms = 1_000;
		do {
			await sleep(timeout_ms);
			timeout_ms = Math.min(timeout_ms * 1.2, 10_000);
			const result = await anonymous.get_transfer({ id: transferId });
			if (DEV) {
				await evmAnonymous(this.evmChain)[0].mine(16);
			}
			switch (true) {
				case 'Ok' in result: {
					const current = result.Ok;
					const status = current.status[0];
					const candidToken = current.destination.token[0];
					if (status === undefined || candidToken == undefined) {
						break;
					}
					const token = fromCandid.token(candidToken);
					this.evmAmount = fromCandid.amount(
						current.destination.amount,
						TOKEN.get(token)!.decimals
					);
					if ('Succeeded' in status) {
						this.evmTx = current.destination.tx[0];
						this.txStatus = 'executed';
					} else if ('Failed' in status) {
						this.update({
							tag: 'err',
							error: status.Failed.error
						});
						return;
					} else {
						for (let entry of current.trace.entries) {
							if (this.evmChain in entry.chain) {
								if (entry.result[0] && 'Ok' in entry.result[0]) {
									const event = entry.event[0];
									if (event === undefined) {
										continue;
									}
									const ts = traceEventToTxStatus(event);
									if (order(this.txStatus) < order(ts)) {
										this.txStatus = ts;
										if (ts === 'executed') {
											this.evmTx = entry.tx[0];
										}
										bridge.tick();
									}
								}
							}
						}
					}
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
					break;
				}
				case 'Err' in result: {
					console.log(`get_transfer failed: ${result.Err}`);
					break;
				}
				default:
					console.log(`unexpected result of get_transfer: ${result}`);
					break;
			}
		} while (this.txStatus != 'executed');

		this.update({
			tag: 'ok'
		});
	}

	private update(status: Status) {
		this._status = status;
		bridge.tick();
	}
}
