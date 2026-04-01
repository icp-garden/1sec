import { EvmUser } from '$lib/user/evmUser';
import { IcpUser } from '$lib/user/icpUser';

export type WalletKind = 'icp' | 'evm';

export interface Wallet {
	name: string;
	icon: string;
	kind: WalletKind;
	isExpired(): Promise<boolean>;
	connect(): Promise<WalletAccount[]>;
	disconnect(): Promise<void>;
}

export interface WalletAccount {
	address(): string;
	connect(): IcpUser | EvmUser;
}
