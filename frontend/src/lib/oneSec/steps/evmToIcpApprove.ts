import { bridge } from '$lib/stores';
import type { Tx } from '../../../declarations/one_sec/one_sec.did';
import type { Step, Contracts, Status, StepTag, EvmChain, Chain } from '../types';
import { txError } from '../utils';

export class EvmToIcpApprove implements Step {
	tag: StepTag = 'EvmToIcpApprove';
	evmChain: EvmChain;
	user: string;
	amount: bigint;
	spender: string;
	_status: Status;

	constructor(evmChain: EvmChain, user: string, amount: bigint, spender: string) {
		this.evmChain = evmChain;
		this.user = user;
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
				return `Approve transaction`;
			case 'pending':
				return `Sign Approve transaction`;
			case 'ok':
				return `Approved transaction`;
			case 'err':
				return `Failed to approve: ${s.error}`;
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
		if (!contracts.erc20) {
			this.update({
				tag: 'err',
				error: 'wallet not connected'
			});
			return;
		}

		try {
			await contracts.evmUser?.switchChain(this.evmChain);
		} catch (err) {
			this.update({
				tag: 'err',
				error: `couldn't to switch wallet to ${this.evmChain}`
			});
			return;
		}

		const erc20 = contracts.erc20;

		this.update({
			tag: 'pending',
			start: new Date(),
			estimatedEnd: null
		});

		try {
			const balance = await erc20.balanceOf(this.user);
			if (balance < this.amount) {
				this.update({
					tag: 'err',
					error: 'insufficient balance'
				});
				return;
			}
		} catch (err) {
			console.warn(err);
			this.update({
				tag: 'err',
				error: 'failed to fetch balance'
			});
			return;
		}

		let receipt;
		try {
			const tx = await erc20.approve(this.spender, this.amount);
			receipt = await tx.wait();
		} catch (err) {
			console.warn(err);
			this.update({
				tag: 'err',
				error: txError(err as Error) // TODO: safely convert to Error
			});
			return;
		}
		if (receipt.status === 1) {
			this.update({
				tag: 'ok'
			});
		} else {
			this.update({
				tag: 'err',
				error: `transaction ${receipt.hash} failed.`
			});
		}
	}

	private update(status: Status) {
		this._status = status;
		bridge.tick();
	}
}
