import { ethers } from 'ethers';
import type { _SERVICE as ICRC2 } from '../../declarations/icrc_ledger/icrc_ledger.did';
import type {
	IcpAccount,
	_SERVICE as OneSec,
	Status as CandidStatus,
	Tx
} from '../../declarations/one_sec/one_sec.did';
import type { _SERVICE as ckUnwrap } from '../../declarations/ck_unwrap/ck_unwrap.did';
import { IcpUser } from '$lib/user/icpUser';
import { EvmUser } from '$lib/user/evmUser';

export type Chain = 'ICP' | EvmChain;
export type EvmChain = 'Base' | 'Arbitrum' | 'Ethereum';
export type Token =
	| 'ICP'
	| 'USDC'
	| 'USDT'
	| 'cbBTC'
	| 'BOB'
	| 'ckBTC'
	| 'GLDT'
	| 'CHAT'
	| 'ckUSDC'
	| 'ckUSDT';
export type OperatingMode = 'minter' | 'locker' | 'unwrapper';
export type BridgeDirection = 'IcpToEvm' | 'EvmToIcp' | 'ckToOneSec';

export const chains: Chain[] = ['Ethereum', 'Base', 'Arbitrum', 'ICP'];

export type AssetKey = string;

export class Asset {
	chain: Chain;
	token: Token;

	constructor(chain: Chain, token: Token) {
		this.chain = chain;
		this.token = token;
	}

	key(): AssetKey {
		return this.chain + ':' + this.token;
	}
}

export interface BridgeRequest {
	direction: BridgeDirection;
	icpAccount: IcpAccount;
	icpToken: Token;
	icpAmount: bigint;
	evmChain: EvmChain;
	evmAccount: string;
	evmToken: Token;
	evmAmount: bigint;
	user: IcpUser | EvmUser;
}

export type Status =
	| { tag: 'planned' }
	| { tag: 'pending'; start: Date; estimatedEnd: Date | null }
	| { tag: 'ok' }
	| { tag: 'err'; error: String };

export type Contracts = {
	oneSec?: OneSec;
	ckUnwrap?: ckUnwrap;
	icrc2?: ICRC2;
	erc20?: ethers.Contract;
	locker?: ethers.Contract;
	evmUser?: EvmUser;
};

export type StepTag =
	| 'EvmToIcpApprove'
	| 'EvmToIcpTransfer'
	| 'EvmToIcpFetch'
	| 'EvmToIcpFinalize'
	| 'IcpToEvmApprove'
	| 'IcpToEvmTransfer'
	| 'IcpToEvmWaitForTx'
	| 'IcpToEvmFinalize'
	| 'IcpToEvmRefund'
	| 'WaitForBlocks'
	| 'ckToOneSec';

export interface Step {
	tag: StepTag;
	label(now: Date): string;
	status(): Status;
	chain(): Chain;
	tx(): Tx | undefined;
	dstAmount(): number | undefined;
	hasSentDstFunds(): boolean;
	refund(): { refund: true; transferId: bigint } | { refund: false };
	run(contracts: Contracts): Promise<void>;
}

export interface TransferInfo {
	toAddress: string;
	fromAddress: string;
	destinationToken: Token;
	sourceToken: Token;
	deposited: number;
	received: number;
	status: CandidStatus;
	ts_ms: bigint | undefined;
	sourceChain: Chain;
	destinationChain: Chain;
	tx: [] | [Tx];
}
