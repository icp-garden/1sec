import {
	CANISTER_ID,
	CANISTER_ID_INTERNET_IDENTITY,
	CANISTER_ID_ONE_SEC_DAPP,
	DEV,
	STAGING
} from '$lib/env';
import { MS_PER_HOUR, NANOS_PER_MS } from '$lib/oneSec/utils';
import { IcpUser } from '$lib/user/icpUser';
import { type Agent, HttpAgent } from '@icp-sdk/core/agent';
import { AuthClient } from '@icp-sdk/auth/client';
import type { Wallet, WalletAccount, WalletKind } from './types';
import { Principal } from '@icp-sdk/core/principal';
import { AccountIdentifier } from '@dfinity/ledger-icp';

const HOST = DEV ? 'http://127.0.1:8080' : 'https://ic0.app';
const DERIVATION_ORIGIN =
	STAGING || DEV ? undefined : `https://${CANISTER_ID_ONE_SEC_DAPP}.icp0.io`;
const IDENTITY_PROVIDER = DEV
	? `http://${CANISTER_ID_INTERNET_IDENTITY}.localhost:8080`
	: 'https://identity.ic0.app';

// Lazy global instance of `AuthAgent`.
let _authClient: AuthClient | undefined = undefined;

async function getAuthClient(): Promise<AuthClient> {
	if (_authClient) {
		return _authClient;
	}
	let authClient = await AuthClient.create({ idleOptions: { idleTimeout: 10 * 60 * 1000 } });
	_authClient = authClient;
	return authClient;
}

export class IIWallet implements Wallet {
	name = 'PassKey';
	icon = '/icons/wallet/astronaut.png';
	kind: WalletKind = 'icp';

	constructor() {
		// Fixing this related issue: https://forum.dfinity.org/t/internet-identity-pop-up-blocked-on-safari/34105/9
		getAuthClient();
	}

	async connect(): Promise<WalletAccount[]> {
		const authClient = await getAuthClient();
		if (await authClient.isAuthenticated()) {
			const identity = authClient.getIdentity();
			const user = identity.getPrincipal();
			const agent = HttpAgent.createSync({
				identity,
				host: HOST
			});
			return [new IIWalletAccount(this, user, agent)];
		}
		return new Promise((resolve, reject) => {
			authClient.login({
				maxTimeToLive: BigInt(MS_PER_HOUR * NANOS_PER_MS),
				allowPinAuthentication: true,
				derivationOrigin: DERIVATION_ORIGIN,
				identityProvider: IDENTITY_PROVIDER,
				onSuccess: async () => {
					const identity = authClient.getIdentity();
					const user = identity.getPrincipal();
					const agent = HttpAgent.createSync({
						identity,
						host: HOST
					});
					resolve([new IIWalletAccount(this, user, agent)]);
				},
				onError: (error) => {
					reject(error);
				}
			});
		});
	}

	async disconnect(): Promise<void> {
		const authClient = await getAuthClient();
		await authClient.logout();
	}

	async isExpired(): Promise<boolean> {
		const authClient = await getAuthClient();
		return !authClient.isAuthenticated();
	}
}

export class IIWalletAccount implements WalletAccount {
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

let wallet = new IIWallet();

export function getWallet(): Wallet {
	return wallet;
}

export async function tryReconnect(): Promise<IcpUser | undefined> {
	const authClient = await getAuthClient();
	if (await authClient.isAuthenticated()) {
		const accounts = await wallet.connect();
		return accounts[0].connect() as IcpUser;
	}
}

let _anonymousAgent: Agent | undefined = undefined;

export function anonymousAgent(): Agent {
	if (_anonymousAgent) {
		return _anonymousAgent;
	}
	const agent = HttpAgent.createSync({
		host: HOST
	});

	if (DEV) {
		agent.fetchRootKey().catch((err) => {
			console.warn('Unable to fetch root key. Check to ensure that your local replica is running');
			console.error(err);
		});
	}

	_anonymousAgent = agent;
	return agent;
}

export function anonymousWallet(): Wallet {
	return {
		name: 'anonymous',
		icon: '',
		kind: 'icp',
		connect: function (): Promise<WalletAccount[]> {
			throw new Error('Should not be called.');
		},
		disconnect: function (): Promise<void> {
			throw new Error('Should not be called.');
		},
		isExpired: function (): Promise<boolean> {
			return Promise.resolve(true);
		}
	};
}
