import { DEV } from '$lib/env';
import { bridge } from '$lib/stores';
import { evmAnonymous } from '$lib/user/evmUser';
import { icpAnonymous } from '$lib/user/icpUser';
import type { Tx } from '../../../declarations/one_sec/one_sec.did';
import type { Chain, Contracts, EvmChain, Status, Step, StepTag } from '../types';
import { ICP_CALL_MS, sleep } from '../utils';
import { TOKEN } from '../config';
import * as fromCandid from '../fromCandid';

export class IcpToEvmFinalize implements Step {
	tag: StepTag = 'IcpToEvmFinalize';
	evmChain: EvmChain;
	evmTx?: Tx;
	emvAmount?: number;
	transferId?: bigint;
	_status: Status;

	constructor(chain: EvmChain) {
		this.evmChain = chain;
		this._status = { tag: 'planned' };
	}

	label(now: Date): string {
		const s = this.status();
		switch (s.tag) {
			case 'planned':
				return `Validate receipt`;
			case 'pending':
				return `Validating receipt`;
			case 'ok':
				return `Validated receipt`;
			case 'err':
				return `Failed to validate: ${s.error}`;
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
		return this.emvAmount;
	}

	hasSentDstFunds(): boolean {
		return false;
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
		while (true) {
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
					this.emvAmount = fromCandid.amount(
						current.destination.amount,
						TOKEN.get(token)!.decimals
					);
					if ('PendingDestinationTx' in status) {
						// Nothing to do, waiting.
						break;
					} else if ('Succeeded' in status) {
						this.evmTx = current.destination.tx[0];
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

			await sleep(timeout_ms);
			timeout_ms = Math.min(timeout_ms * 1.2, 10_000);
		}
	}

	private update(status: Status) {
		this._status = status;
		bridge.tick();
	}
}
