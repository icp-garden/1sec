import { PostMessageTransport } from '@slide-computer/signer-web';
import type { Wallet, WalletAccount, WalletKind } from './types';
import { Signer } from '@slide-computer/signer';
import { SignerAgent } from '@slide-computer/signer-agent';
import { IcpUser } from '$lib/user/icpUser';
import { Principal } from '@icp-sdk/core/principal';
import type { Agent } from '@icp-sdk/core/agent';
import { AccountIdentifier } from '@dfinity/ledger-icp';

export const NFID_RPC = 'https://nfid.one/rpc';

export class NFIDWallet implements Wallet {
	name = 'Google | NFID';
	icon = '/icons/wallet/google.svg';
	kind: WalletKind = 'icp';

	async connect(): Promise<NFIDWalletAccount[]> {
		const transport = new PostMessageTransport({
			url: NFID_RPC,
			detectNonClickEstablishment: false
		});
		const signer = new Signer({ transport });
		const accounts = await signer.accounts();
		const result = [];
		for (let account of accounts) {
			const agent = SignerAgent.createSync({
				signer,
				account: account.owner
			});
			result.push(new NFIDWalletAccount(this, account.owner, agent));
		}
		return result;
	}

	async disconnect(): Promise<void> {
		// No op.
	}

	async isExpired(): Promise<boolean> {
		return false;
	}
}

export class NFIDWalletAccount implements WalletAccount {
	wallet: Wallet;
	user: Principal;
	accountId: AccountIdentifier;
	agent: Agent;

	constructor(wallet: Wallet, user: Principal, agent: Agent) {
		this.wallet = wallet;
		this.user = user;
		this.accountId = AccountIdentifier.fromPrincipal({ principal: user });
		this.agent = agent;
	}

	address(): string {
		return this.user.toText();
	}

	connect(): IcpUser {
		return new IcpUser(this.user, this.accountId, this.agent, this.wallet);
	}
}

let wallet = new NFIDWallet();

export function getWallet(): Wallet {
	return wallet;
}
