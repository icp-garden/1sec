import type { TransferArg, Tx } from '../../../declarations/one_sec/one_sec.did';
import type { BridgeRequest, Chain, Contracts, EvmChain, Status, Step, StepTag } from '../types';
import * as candid from '../toCandid';
import { ICP_CALL_MS, MS_PER_SECOND, sleep } from '../utils';
import { icpAnonymous } from '$lib/user/icpUser';
import { DEV } from '$lib/env';
import { evmAnonymous } from '$lib/user/evmUser';
import { bridge } from '$lib/stores';
import { EvmToIcpFinalize } from './evmToIcpFinalize';

export class EvmToIcpFetch implements Step {
	tag: StepTag = 'EvmToIcpFetch';
	request: TransferArg;
	blockNumber?: number;
	firstBlockNumber?: number;
	lastBlockNumber?: number;
	transferId?: bigint;
	evmChain: EvmChain;
	_status: Status;
	finalize: EvmToIcpFinalize;

	constructor(request: BridgeRequest, finalize: EvmToIcpFinalize) {
		this.evmChain = request.evmChain;
		this.request = candid.transferRequest(request);
		this._status = { tag: 'planned' };
		this.finalize = finalize;
	}

	label(now: Date): string {
		const s = this.status();
		switch (s.tag) {
			case 'planned':
				return 'Validated receipt';
			case 'pending': {
				if (
					this.blockNumber === undefined ||
					this.firstBlockNumber === undefined ||
					this.lastBlockNumber === undefined
				) {
					return 'Validating receipt: 0%';
				}
				const total = Math.max(this.blockNumber - this.firstBlockNumber, 1);
				const current = Math.min(Math.max(this.lastBlockNumber - this.firstBlockNumber, 0), total);
				const percentBlock = current / (total * 1.5);
				const percentTime = !s.estimatedEnd
					? 1
					: Math.min(
							(now.getTime() - s.start.getTime()) / (s.estimatedEnd.getTime() - s.start.getTime()),
							1
						);
				const delta = percentBlock / 10;
				const percent =
					percentTime < 0.9 + delta ? percentTime : Math.max(0.9 + delta, percentBlock);
				return `Validating receipt: ${Math.min(Math.round(percent * 100), 99)}%`;
			}
			case 'ok':
				return 'Validated receipt';
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
		return this.request.source.tx[0];
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

	notify(tx: Tx, blockNumber?: number) {
		this.request.source.tx = [tx];
		this.blockNumber = blockNumber;
	}

	async run(contracts: Contracts) {
		if (!this.request.source.tx) {
			this.update({
				tag: 'err',
				error: 'missing transaction'
			});
		}

		const oneSec = icpAnonymous().oneSec();

		const start = new Date();
		const estimatedEnd = new Date(start.getTime() + 16 * MS_PER_SECOND);
		this.update({
			tag: 'pending',
			start,
			estimatedEnd: estimatedEnd
		});

		let lastCall: Date | undefined = undefined;
		let timeout_ms = 1_000;
		while (true) {
			const now = new Date();
			if (!lastCall || now.getTime() - lastCall.getTime() >= timeout_ms) {
				lastCall = now;
				timeout_ms = Math.min(timeout_ms * 1.2, 10_000);
				if (DEV) {
					await evmAnonymous(this.evmChain)[0].mine(16);
				}
				const result = await oneSec.transfer(this.request);
				switch (true) {
					case 'Accepted' in result:
						this.transferId = result.Accepted.id;
						break;
					case 'Failed' in result:
						this.update({
							tag: 'err',
							error: result.Failed.error
						});
						return;
					case 'Fetching' in result:
						this.lastBlockNumber = Number(result.Fetching.block_height);
						if (this.firstBlockNumber === undefined) {
							this.firstBlockNumber = this.lastBlockNumber;
						}
						break;
				}
			}

			if (this.transferId != null) {
				break;
			}

			bridge.tick();
			await sleep(1_000);
		}

		const transferId = this.transferId as bigint;
		this.finalize.notify(transferId);

		this.update({
			tag: 'ok'
		});
	}

	private update(status: Status) {
		this._status = status;
		bridge.tick();
	}
}
