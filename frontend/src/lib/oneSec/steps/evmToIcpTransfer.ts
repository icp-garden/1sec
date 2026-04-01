import type { IcpAccount, Tx } from '../../../declarations/one_sec/one_sec.did';
import { ethers } from 'ethers';
import { encodeIcpAccount, displayValue } from '$lib/utils';
import type {
	Chain,
	Contracts,
	EvmChain,
	OperatingMode,
	Status,
	Step,
	StepTag,
	Token
} from '../types';
import { bridge } from '$lib/stores';
import { txError } from '../utils';
import { EvmToIcpFetch } from './evmToIcpFetch';
import { tokenToDecimals } from '../config';

export class EvmToIcpTransfer implements Step {
	tag: StepTag = 'EvmToIcpTransfer';
	evmChain: EvmChain;
	evmTx?: Tx;
	user: string;
	mode: OperatingMode;
	amount: bigint;
	tokenName: Token;
	receiver: IcpAccount;
	blockNumber?: number;
	fetch: EvmToIcpFetch;
	_status: Status;

	constructor(
		evmChain: EvmChain,
		user: string,
		mode: OperatingMode,
		amount: bigint,
		tokenName: Token,
		receiver: IcpAccount,
		fetch: EvmToIcpFetch
	) {
		this.tokenName = tokenName;
		this.evmChain = evmChain;
		this.user = user;
		this.mode = mode;
		this.amount = amount;
		this.receiver = receiver;
		this.fetch = fetch;
		this._status = { tag: 'planned' };
	}

	label(now: Date): string {
		const s = this.status();
		switch (s.tag) {
			case 'planned':
				return `Execute transaction`;
			case 'pending':
				return `Sign transfer transaction`;
			case 'ok':
				return `Transferred ${Number(this.amount) / Math.pow(10, tokenToDecimals(this.tokenName))} ${this.tokenName} to OneSec`;
			case 'err':
				return `Failed to transfer ${this.tokenName}: ${s.error}`;
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
		return undefined;
	}

	hasSentDstFunds(): boolean {
		return false;
	}

	refund(): { refund: false } {
		return { refund: false };
	}

	async run(contracts: Contracts) {
		this.update({
			tag: 'pending',
			start: new Date(),
			estimatedEnd: null
		});

		try {
			await contracts.evmUser?.switchChain(this.evmChain);
		} catch (err) {
			this.update({
				tag: 'err',
				error: `couldn't to switch wallet to ${this.evmChain}`
			});
			return;
		}

		switch (this.mode) {
			case 'locker': {
				if (contracts.locker) {
					await this.lock(contracts.locker);
				} else {
					this.update({
						tag: 'err',
						error: 'wallet not connected'
					});
				}
				break;
			}
			case 'minter': {
				if (contracts.erc20) {
					await this.burn(contracts.erc20);
				} else {
					this.update({
						tag: 'err',
						error: 'wallet not connected'
					});
				}
				break;
			}
		}
	}

	async lock(locker: ethers.Contract) {
		let receipt;
		try {
			const tx = await locker.lock1(this.amount, encodeIcpAccount(this.receiver));
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
			this.evmTx = { Evm: { log_index: [], hash: receipt.hash } };
			this.blockNumber = receipt.blockNumber;
			this.fetch.notify(this.evmTx, this.blockNumber);
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

	async burn(minter: ethers.Contract) {
		try {
			const balance = await minter.balanceOf(this.user);
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
			const tx = await minter.burn1(this.amount, encodeIcpAccount(this.receiver));
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
			this.evmTx = { Evm: { log_index: [], hash: receipt.hash } };
			this.blockNumber = receipt.blockNumber;
			this.fetch.notify(this.evmTx, this.blockNumber);
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
