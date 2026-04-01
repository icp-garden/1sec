import { ckToOneSec } from './steps/ckToOneSec';
import { IcpToEvmApprove } from './steps/icpToEvmApprove';
import { IcpToEvmTransfer } from './steps/icpToEvmTransfer';
import { IcpToEvmWaitForTx } from './steps/icpToEvmWaitForTx';
import { IcpToEvmFinalize } from './steps/icpToEvmFinalize';
import { WaitForBlocks } from './steps/waitForBlocks';
import { EvmToIcpApprove } from './steps/evmToIcpApprove';
import { EvmToIcpTransfer } from './steps/evmToIcpTransfer';
import { EvmToIcpFinalize } from './steps/evmToIcpFinalize';
import type { BridgeRequest, Chain, Contracts, Status, Step, Token, EvmChain } from './types';
import type { EvmConfig, EvmTokenConfig } from './config';
import { CONFIG, TOKEN } from './config';
import { bridge } from '$lib/stores';
import { EvmToIcpFetch } from './steps/evmToIcpFetch';
import type { Tx } from '../../declarations/one_sec/one_sec.did';
import * as fromCandid from './fromCandid';
import { IcpToEvmRefund } from './steps/icpToEvmRefund';

export interface StepDetails {
	index: number;
	label: string;
	status: Status;
	chain: Chain;
	tx?: Tx;
	dstAmount?: number;
	hasSentDstFunds: boolean;
	refund: boolean;
}

export class Bridge {
	request: BridgeRequest;
	currentStep: number;
	_steps: Step[];
	start?: Date;

	constructor(request: BridgeRequest) {
		this.request = request;
		this.currentStep = 0;
		switch (request.direction) {
			case 'IcpToEvm':
				this._steps = icpToEvm(request);
				break;
			case 'EvmToIcp':
				this._steps = evmToIcp(request);
				break;
			case 'ckToOneSec':
				this._steps = ckToOneSecUSD(request);
				break;
		}
	}

	done(): boolean {
		if (this.currentStep === this._steps.length) {
			return true;
		}
		const status = this._steps[this.currentStep].status();
		return status.tag === 'err';
	}

	succeeded(): boolean {
		if (this.currentStep === this._steps.length) {
			return this._steps[this.currentStep - 1].status().tag === 'ok';
		}
		return false;
	}

	steps(): StepDetails[] {
		const now = new Date();
		const result: StepDetails[] = [];
		for (let i = 0; i <= this.currentStep && i < this._steps.length; ++i) {
			result.push({
				index: i,
				label: this._steps[i].label(now),
				status: this._steps[i].status(),
				chain: this._steps[i].chain(),
				tx: this._steps[i].tx(),
				dstAmount: this._steps[i].dstAmount(),
				hasSentDstFunds: this._steps[i].hasSentDstFunds(),
				refund: this._steps[i].refund().refund
			});
		}
		return result;
	}

	dstTx(): Tx | undefined {
		if (this.currentStep === this._steps.length) {
			return this._steps[this.currentStep - 1].tx();
		}
		return undefined;
	}

	dstAmount(): number | undefined {
		if (this.currentStep === this._steps.length) {
			return this._steps[this.currentStep - 1].dstAmount();
		}
		const amount = this._steps[this.currentStep].dstAmount();
		if (amount === undefined && this.currentStep > 0) {
			return this._steps[this.currentStep - 1].dstAmount();
		}
		return amount;
	}

	dstToken(): Token {
		switch (this.request.direction) {
			case 'IcpToEvm': {
				return this.request.evmToken;
			}
			case 'EvmToIcp': {
				return this.request.icpToken;
			}
			case 'ckToOneSec': {
				return this.request.evmToken;
			}
		}
	}

	dstChain(): Chain {
		switch (this.request.direction) {
			case 'IcpToEvm': {
				return this.request.evmChain;
			}
			case 'EvmToIcp': {
				return 'ICP';
			}
			case 'ckToOneSec': {
				return 'ICP';
			}
		}
	}

	refund(): boolean {
		for (let step of this._steps) {
			if (step.refund().refund) {
				return true;
			}
		}
		return false;
	}

	refundTx(): Tx | undefined {
		if (this.refund() && this.currentStep === this._steps.length) {
			return this._steps[this.currentStep - 1].tx();
		}
		return undefined;
	}

	srcAmount(): number {
		switch (this.request.direction) {
			case 'EvmToIcp': {
				const amount = this.request.evmAmount;
				const decimals = TOKEN.get(this.request.evmToken)!.decimals;
				return fromCandid.amount(amount, decimals);
			}
			case 'IcpToEvm': {
				const amount = this.request.icpAmount;
				const decimals = TOKEN.get(this.request.icpToken)!.decimals;
				return fromCandid.amount(amount, decimals);
			}
			case 'ckToOneSec': {
				const amount = this.request.icpAmount;
				const decimals = TOKEN.get(this.request.icpToken)!.decimals;
				return fromCandid.amount(amount, decimals);
			}
		}
	}

	srcToken(): Token {
		switch (this.request.direction) {
			case 'EvmToIcp': {
				return this.request.evmToken;
			}
			case 'IcpToEvm': {
				return this.request.icpToken;
			}
			case 'ckToOneSec': {
				return this.request.icpToken;
			}
		}
	}

	srcChain(): Chain {
		switch (this.request.direction) {
			case 'EvmToIcp': {
				return this.request.evmChain;
			}
			case 'IcpToEvm': {
				return 'ICP';
			}
			case 'ckToOneSec': {
				return 'ICP';
			}
		}
	}

	async run(contracts: Contracts) {
		this.start = new Date();
		while (this.currentStep < this._steps.length) {
			const step = this._steps[this.currentStep];
			await step.run(contracts);
			const status = step.status();
			switch (status.tag) {
				case 'ok': {
					++this.currentStep;
					const refund = step.refund();
					if (refund.refund) {
						if (this.request.direction !== 'IcpToEvm') {
							throw new Error('unexpected refund');
						}
						const transferId = refund.transferId;
						this._steps[this.currentStep] = new IcpToEvmRefund(transferId);
						this._steps.length = this.currentStep + 1;
					}
					break;
				}
				case 'err': {
					return;
				}
				case 'pending':
				case 'planned': {
					throw new Error(`step has not completed after run: ${step}`);
				}
			}
		}
		bridge.tick();
	}
}

function icpToEvm(r: BridgeRequest): Step[] {
	const waitForTx = new IcpToEvmWaitForTx(r.evmChain);
	const finalize = new IcpToEvmFinalize(r.evmChain);
	const evm = CONFIG.evm.get(r.evmChain) as EvmConfig;

	let principal;

	if ('ICRC' in r.icpAccount) {
		principal = r.icpAccount.ICRC.owner;
	} else if ('AccountId' in r.icpAccount) {
		throw Error(
			`Account identifier ${r.icpAccount.AccountId} is not supported in ICP to EVM bridging`
		);
	} else {
		const _unreachable: never = r.icpAccount;
		throw Error('unreachable');
	}

	return [
		new IcpToEvmApprove(principal, r.icpToken, r.icpAmount, CONFIG.icp.oneSec),
		new IcpToEvmTransfer(r, waitForTx, finalize, r.icpAmount, r.icpToken),
		waitForTx,
		new WaitForBlocks(r.evmChain, Math.round(evm.safetyMargin * 1.2), evm.blockTimeMs),
		finalize
	];
}

function evmToIcp(r: BridgeRequest): Step[] {
	const evm = CONFIG.evm.get(r.evmChain) as EvmConfig;
	const token = evm.token.get(r.evmToken) as EvmTokenConfig;
	const chain = r.evmChain;
	const mode = token.mode;

	const finalize = new EvmToIcpFinalize(r);
	const fetch = new EvmToIcpFetch(r, finalize);
	const transfer = new EvmToIcpTransfer(
		chain,
		r.evmAccount,
		mode,
		r.evmAmount,
		r.evmToken,
		r.icpAccount,
		fetch
	);
	const waitForBlocks = new WaitForBlocks(
		chain,
		Math.round(evm.safetyMargin * 1.2),
		evm.blockTimeMs
	);

	switch (mode) {
		case 'minter':
			return [transfer, waitForBlocks, fetch, finalize];
		case 'locker':
			return [
				new EvmToIcpApprove(chain, r.evmAccount, r.evmAmount, token.locker as string),
				transfer,
				waitForBlocks,
				fetch,
				finalize
			];
	}
}

function ckToOneSecUSD(r: BridgeRequest): Step[] {
	const evm = CONFIG.evm.get('Ethereum') as EvmConfig;

	const bridge: Step = new ckToOneSec(r);
	const waitForBlocks = new WaitForBlocks('Ethereum', Math.round(40 * 1.2), evm.blockTimeMs);

	let principal;

	if ('ICRC' in r.icpAccount) {
		principal = r.icpAccount.ICRC.owner;
	} else if ('AccountId' in r.icpAccount) {
		throw Error(
			`Account identifier ${r.icpAccount.AccountId} is not supported in ck to OneSec bridging`
		);
	} else {
		const _unreachable: never = r.icpAccount;
		throw Error('unreachable');
	}

	return [
		new IcpToEvmApprove(principal, r.icpToken, r.icpAmount * 2n, CONFIG.icp.ckUnwrap),
		bridge,
		waitForBlocks
	];
}
