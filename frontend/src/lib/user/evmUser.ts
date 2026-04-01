import { DEV } from '$lib/env';
import { ABI, CONFIG, TOKEN, type EvmConfig } from '$lib/oneSec/config';
import { Asset } from '$lib/oneSec/types';
import type { Chain, EvmChain, Token } from '$lib/oneSec/types';
import { type Balance } from '$lib/types';
import type { Wallet } from '$lib/wallet/types';
import { Contract, type JsonRpcSigner, JsonRpcProvider } from 'ethers';
import * as fromCandid from '$lib/oneSec/fromCandid';
import { ethers } from 'ethers';

export class EvmUser {
	address: string;
	signer: JsonRpcSigner;
	wallet: Wallet;
	_erc20: Map<EvmChain, Map<Token, Contract>>;
	_locker: Map<EvmChain, Map<Token, Contract>>;
	_balance: Map<EvmChain, Map<Token, Balance>>;
	addedDevChain: Set<EvmChain>;

	constructor(address: string, signer: JsonRpcSigner, wallet: Wallet) {
		this.address = address;
		this.signer = signer;
		this.wallet = wallet;
		this._erc20 = new Map();
		this._locker = new Map();
		this._balance = new Map();
		this.addedDevChain = new Set();
	}

	erc20(chain: EvmChain, token: Token): Contract | undefined {
		let map = this._erc20.get(chain);
		if (!map) {
			map = new Map();
			this._erc20.set(chain, map);
		}
		if (map.has(token)) {
			return map.get(token);
		}
		const config = CONFIG.evm.get(chain)?.token.get(token);
		if (!config) {
			console.error(`no EVM config for ${chain} and ${token}`);
			return undefined;
		}
		const abi = config.mode === 'minter' ? ABI.erc20_and_minter : ABI.erc20;
		const erc20 = new Contract(config.erc20, abi, this.signer);
		map.set(token, erc20);
		return erc20;
	}

	locker(chain: EvmChain, token: Token): Contract | undefined {
		let map = this._locker.get(chain);
		if (!map) {
			map = new Map();
			this._locker.set(chain, map);
		}
		if (map.has(token)) {
			return map.get(token);
		}
		const config = CONFIG.evm.get(chain)?.token.get(token);
		if (!config || !config.locker) {
			return undefined;
		}
		const locker = new Contract(config.locker, ABI.locker, this.signer);
		map.set(token, locker);
		return locker;
	}

	async switchChain(chain: EvmChain) {
		const config = CONFIG.evm.get(chain);
		if (!config) {
			console.error(`no EVM config for ${chain}`);
			return;
		}
		const chainId = config.chainId;
		const hexChainId = '0x' + chainId.toString(16);

		try {
			await this.signer.provider.send('wallet_switchEthereumChain', [{ chainId: hexChainId }]);
		} catch (err) {
			if (
				DEV &&
				(chainId === 31337 || chainId === 31338 || chainId === 31339) &&
				!this.addedDevChain.has(chain)
			) {
				this.addedDevChain.add(chain);
				await this.signer.provider.send('wallet_addEthereumChain', [
					{
						chainId: hexChainId,
						chainName: 'Local Network',
						rpcUrls: [config.rpcUrl],
						nativeCurrency: {
							name: 'Ether',
							symbol: 'ETH',
							decimals: 18
						}
					}
				]);
			}
			await this.signer.provider.send('wallet_switchEthereumChain', [{ chainId: hexChainId }]);
			throw err;
		}
	}

	assets(): Asset[] {
		const result = [];
		for (let chain of CONFIG.evm.keys()) {
			for (let token of CONFIG.evm.get(chain)!.token.keys()) {
				result.push(new Asset(chain, token));
			}
		}
		return result;
	}

	tokens(): Token[] {
		return [...new Set(this.assets().map((x) => x.token))];
	}

	chains(token: Token): Chain[] {
		return [
			...CONFIG.evm
				.entries()
				.filter(([chain, config]) => config.token.has(token))
				.map(([chain]) => chain)
		];
	}

	getBalance(chain: Chain, token: Token): Balance | undefined {
		return this._balance.get(chain as EvmChain)?.get(token);
	}

	async fetchBalance(chain: EvmChain, token: Token) {
		const amount = await fetchBalance(chain, token, this.address);
		if (!this._balance.has(chain)) {
			this._balance.set(chain, new Map());
		}
		this._balance.get(chain)!.set(token, { amount, lastUpdated: new Date() });
	}
}

export class EvmAnonymous {
	_provider: JsonRpcProvider;
	_erc20: Map<EvmChain, Map<Token, Contract>>;
	_url: string;

	constructor(provider: JsonRpcProvider, url: string) {
		this._provider = provider;
		this._erc20 = new Map();
		this._url = url;
	}

	provider(): JsonRpcProvider {
		return this._provider;
	}

	url(): string {
		return this._url;
	}

	erc20(chain: EvmChain, token: Token): Contract | undefined {
		let map = this._erc20.get(chain);
		if (!map) {
			map = new Map();
			this._erc20.set(chain, map);
		}
		if (map.has(token)) {
			return map.get(token);
		}
		const config = CONFIG.evm.get(chain)?.token.get(token);
		if (!config) {
			console.error(`no EVM config for ${chain} and ${token}`);
			return undefined;
		}
		const abi = config.mode === 'minter' ? ABI.erc20_and_minter : ABI.erc20;
		const erc20 = new Contract(config.erc20, abi, this._provider);
		map.set(token, erc20);
		return erc20;
	}

	async mine(blocks: number) {
		for (let i = 0; i < blocks; i++) {
			await this._provider.send('anvil_mine', []);
		}
	}

	async gasCost(): Promise<number> {
		const wei = await this._provider.send('eth_gasPrice', []);
		return Number(ethers.formatEther(wei));
	}
}

const _evmAnonymous: Map<EvmChain, EvmAnonymous[]> = new Map();
export function evmAnonymous(chain: EvmChain): EvmAnonymous[] {
	let result = _evmAnonymous.get(chain);
	if (result) {
		return result;
	}
	const config = CONFIG.evm.get(chain) as EvmConfig;
	result = [];
	for (let rpcUrl of config.rpcUrl) {
		const provider = new JsonRpcProvider(rpcUrl);
		result.push(new EvmAnonymous(provider, rpcUrl));
	}
	_evmAnonymous.set(chain, result);
	return result;
}

export async function fetchBalance(
	chain: EvmChain,
	token: Token,
	address: string
): Promise<number> {
	const nodes = evmAnonymous(chain);

	let errors = [];

	for (let node of nodes) {
		try {
			const erc20 = node.erc20(chain, token);
			if (erc20 === undefined) {
				errors.push({
					node: node.url(),
					error: `couldn't find ERC20 for ${chain} / ${token}`
				});
				continue;
			}
			const amount = await erc20!.balanceOf(address);
			return fromCandid.amount(amount, TOKEN.get(token)!.decimals);
		} catch (e) {
			errors.push({
				node: node.url(),
				error: `failed to fetch balance: ${e}`
			});
		}
	}

	throw Error(`failed to fetch balance ${chain}/${token}: ${errors[0].error}`);
}
