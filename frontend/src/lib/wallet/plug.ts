import type { Wallet, WalletAccount, WalletKind } from './types';
import { Signer } from '@slide-computer/signer';
import { SignerAgent } from '@slide-computer/signer-agent';
import { IcpUser } from '$lib/user/icpUser';
import { BrowserExtensionTransport } from '@slide-computer/signer-extension';
import { Principal } from '@icp-sdk/core/principal';
import type { Agent } from '@icp-sdk/core/agent';
import { ethers } from 'ethers';
import type { Eip1193Provider } from 'ethers';
import type { JsonRpcSigner } from 'ethers';
import { walletDetails } from './evm';
import { AccountIdentifier } from '@dfinity/ledger-icp';

export const NFID_RPC = 'https://nfid.one/rpc';

export class PlugWallet implements Wallet {
	name = 'Plug';
	icon = '/icons/wallet/plug.png';
	kind: WalletKind = 'icp';
	provider: Eip1193Provider;

	constructor(provider: Eip1193Provider) {
		this.provider = provider;
	}

	async connect(): Promise<PlugWalletAccount[]> {
		const evmSigner = await this.evmConnect();
		const accounts = await this.icpConnect();

		return accounts.map(([user, agent]) => {
			return new PlugWalletAccount(this, user, agent, evmSigner);
		});
	}

	async evmConnect(): Promise<JsonRpcSigner> {
		const accountProvider = new ethers.BrowserProvider(this.provider, 'any');
		const signer = await accountProvider.getSigner();
		return signer;
	}

	async icpConnect(): Promise<[Principal, SignerAgent][]> {
		const transport = await BrowserExtensionTransport.findTransport({
			uuid: '71edc834-bab2-4d59-8860-c36a01fee7b8'
		});

		const icpSigner = new Signer({ transport });
		await icpSigner.requestPermissions([
			{ method: 'icrc27_accounts' },
			{ method: 'icrc49_call_canister' }
		]);
		const accounts = await icpSigner.accounts();
		const result = [];
		for (let account of accounts) {
			const agent = SignerAgent.createSync({
				signer: icpSigner,
				account: account.owner
			});
			result.push([account.owner, agent] as [Principal, SignerAgent]);
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

export class PlugWalletAccount implements WalletAccount {
	wallet: Wallet;
	user: Principal;
	accountId: AccountIdentifier;
	agent: Agent;
	signer: JsonRpcSigner;

	constructor(wallet: Wallet, user: Principal, agent: Agent, signer: JsonRpcSigner) {
		this.wallet = wallet;
		this.user = user;
		this.accountId = AccountIdentifier.fromPrincipal({ principal: user });
		this.agent = agent;
		this.signer = signer;
	}

	address(): string {
		return this.user.toText();
	}

	connect(): IcpUser {
		return new IcpUser(this.user, this.accountId, this.agent, this.wallet);
	}
}

// TODO maybe fix the detail.info.name
export function getWallet(): Wallet | undefined {
	const plugDetail = walletDetails.filter((detail) => detail.info.name === 'Plug wallet')[0];
	return plugDetail ? new PlugWallet(plugDetail.provider) : undefined;
}
