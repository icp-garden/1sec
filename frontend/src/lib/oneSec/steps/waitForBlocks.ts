import { bridge } from '$lib/stores';
import type { Tx } from '../../../declarations/one_sec/one_sec.did';
import type { Step, Contracts, Status, StepTag, Chain, EvmChain } from '../types';
import { MS_PER_SECOND, sleep } from '../utils';

export class WaitForBlocks implements Step {
	tag: StepTag = 'WaitForBlocks';
	evmChain: EvmChain;
	blockCount: number;
	blockTimeMs: number;
	_status: Status;

	constructor(evmChain: EvmChain, blockCount: number, blockTimeMs: number) {
		this.evmChain = evmChain;
		this.blockCount = blockCount;
		this.blockTimeMs = blockTimeMs;
		this._status = { tag: 'planned' };
	}

	label(now: Date): string {
		const total = this.blockCount;
		const s = this.status();
		switch (s.tag) {
			case 'planned':
				return `Confirm ${this.blockCount} blocks`;
			case 'pending': {
				const elapsedMs = now.getTime() - s.start.getTime();
				const blocks = Math.floor(elapsedMs / this.blockTimeMs);
				if (blocks >= total) {
					return `Confirmed blocks`;
				}
				return `Confirming blocks: ${blocks}/${total}`;
			}
			case 'ok':
				return `Confirmed blocks`;
			case 'err':
				return `Failed to confirm blocks: ${s.error}`;
		}
	}

	status(): Status {
		return this._status;
	}

	chain(): Chain {
		return this.evmChain;
	}

	tx(): Tx | undefined {
		return undefined;
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
		const start = new Date();
		const duration = this.blockCount * this.blockTimeMs;
		const estimatedEnd = new Date(start.getTime() + duration);
		this.update({
			tag: 'pending',
			start,
			estimatedEnd
		});

		while (true) {
			const now = new Date();
			if (now >= estimatedEnd) {
				break;
			}
			bridge.tick();
			await sleep(Math.min(MS_PER_SECOND, estimatedEnd.getTime() - now.getTime()));
		}

		this.update({
			tag: 'ok'
		});
	}

	private update(status: Status) {
		this._status = status;
		bridge.tick();
	}
}
