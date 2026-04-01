// Keep this in sync with `one_sec/src/config.rs`.

import { CANISTER_ID_ONE_SEC, DEV, STAGING, CANISTER_ID_CK_UNWRAP } from '$lib/env';
import { Principal } from '@dfinity/principal';
import type { EvmChain, Token, OperatingMode, Chain } from './types';
import { Asset } from './types';

export interface IcpConfig {
	oneSec: Principal;
	ckUnwrap: Principal;
	logoPath: string;
	token: Map<Token, IcpTokenConfig>;
}

export interface IcpTokenConfig {
	mode: OperatingMode;
	ledger: Principal;
}

export interface EvmConfig {
	chainId: number;
	rpcUrl: string[];
	logoPath: string;
	safetyMargin: number;
	blockTimeMs: number;
	token: Map<Token, EvmTokenConfig>;
}

export interface EvmTokenConfig {
	mode: OperatingMode;
	erc20: string;
	locker?: string;
}

export interface Config {
	icp: IcpConfig;
	evm: Map<EvmChain, EvmConfig>;
}

export interface TokenConfig {
	decimals: number;
	logoPath: string;
	ledgerFee: number;
}

const LOCAL_EVM_TOKENS: Map<Token, EvmTokenConfig> = new Map([
	[
		'ICP',
		{
			mode: 'minter',
			erc20: '0x5FbDB2315678afecb367f032d93F642f64180aa3'
		}
	],
	[
		'USDC',
		{
			mode: 'locker',
			erc20: '0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512',
			locker: '0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0'
		}
	]
]);

const STAGING_ETH_EVM_TOKENS: Map<Token, EvmTokenConfig> = new Map([
	[
		'ICP',
		{
			mode: 'minter',
			erc20: '0xeBC37fa86e87C912B3f7b98FF0211992EDF42257'
		}
	],
	[
		'USDC',
		{
			mode: 'locker',
			erc20: '0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48',
			locker: '0xd060B59875c7eD702D48f4c35a122191379D4f85'
		}
	],
	[
		'USDT',
		{
			mode: 'locker',
			erc20: '0xdAC17F958D2ee523a2206206994597C13D831ec7',
			locker: '0x205E3f1001bbE91971D25349ac3aA949D9Be5079'
		}
	],
	[
		'cbBTC',
		{
			mode: 'locker',
			erc20: '0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf',
			locker: '0xd543007D8415169756e8a61b2cc079369d4aB6a8'
		}
	],
	[
		'ckBTC',
		{
			mode: 'minter',
			erc20: '0x9D8dE8E7Cd748F760C81199AD3b902798DA7E7bC'
		}
	],
	[
		'GLDT',
		{
			mode: 'minter',
			erc20: '0xB5A497b709703eC987B6879f064B02017998De1d'
		}
	],
	[
		'BOB',
		{
			mode: 'minter',
			erc20: '0xc6d02fa25bC437E38099476a6856225aE5ac2C75'
		}
	]
]);

const STAGING_ARB_EVM_TOKENS: Map<Token, EvmTokenConfig> = new Map([
	[
		'ICP',
		{
			mode: 'minter',
			erc20: '0xC79221a2152136FE680f86562D0659706d23946A'
		}
	],
	[
		'USDC',
		{
			mode: 'locker',
			erc20: '0xaf88d065e77c8cC2239327C5EDb3A432268e5831',
			locker: '0x3a9238e29Fe809df8f392e4DfB8606EB102C5e98'
		}
	],
	[
		'cbBTC',
		{
			mode: 'locker',
			erc20: '0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf',
			locker: '0xd543007D8415169756e8a61b2cc079369d4aB6a8'
		}
	],
	[
		'ckBTC',
		{
			mode: 'minter',
			erc20: '0x9D8dE8E7Cd748F760C81199AD3b902798DA7E7bC'
		}
	],
	[
		'GLDT',
		{
			mode: 'minter',
			erc20: '0xB5A497b709703eC987B6879f064B02017998De1d'
		}
	],
	[
		'BOB',
		{
			mode: 'minter',
			erc20: '0xc6d02fa25bC437E38099476a6856225aE5ac2C75'
		}
	]
]);

const STAGING_BASE_EVM_TOKENS: Map<Token, EvmTokenConfig> = new Map([
	[
		'ICP',
		{
			mode: 'minter',
			erc20: '0xa96496d9Ef442a3CF8F3e24B614b87a70ddf74f3'
		}
	],
	[
		'USDC',
		{
			mode: 'locker',
			erc20: '0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913',
			locker: '0x38200DD4c3adbE86Be49717ccA8a3fD08466Cba6'
		}
	],
	[
		'cbBTC',
		{
			mode: 'locker',
			erc20: '0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf',
			locker: '0xd543007D8415169756e8a61b2cc079369d4aB6a8'
		}
	],
	[
		'ckBTC',
		{
			mode: 'minter',
			erc20: '0x9D8dE8E7Cd748F760C81199AD3b902798DA7E7bC'
		}
	],
	[
		'GLDT',
		{
			mode: 'minter',
			erc20: '0xB5A497b709703eC987B6879f064B02017998De1d'
		}
	],
	[
		'BOB',
		{
			mode: 'minter',
			erc20: '0xc6d02fa25bC437E38099476a6856225aE5ac2C75'
		}
	]
]);

const MAINNET_BASE_EVM_TOKENS: Map<Token, EvmTokenConfig> = new Map([
	[
		'ICP',
		{
			mode: 'minter',
			erc20: '0x00f3C42833C3170159af4E92dbb451Fb3F708917'
		}
	],
	[
		'USDC',
		{
			mode: 'locker',
			erc20: '0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913',
			locker: '0xAe2351B15cFf68b5863c6690dCA58Dce383bf45A'
		}
	],
	[
		'cbBTC',
		{
			mode: 'locker',
			erc20: '0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf',
			locker: '0x7744c6a83E4b43921f27d3c94a742bf9cd24c062'
		}
	],
	[
		'ckBTC',
		{
			mode: 'minter',
			erc20: '0x919A41Ea07c26f0001859Bc5dcb8754068718Fb7'
		}
	],
	[
		'GLDT',
		{
			mode: 'minter',
			erc20: '0x86856814e74456893Cfc8946BedcBb472b5fA856'
		}
	],
	[
		'BOB',
		{
			mode: 'minter',
			erc20: '0xecc5f868AdD75F4ff9FD00bbBDE12C35BA2C9C89'
		}
	],
	[
		'CHAT',
		{
			mode: 'minter',
			erc20: '0xDb95092C454235E7e666c4E226dBBbCdeb499d25'
		}
	]
]);

const MAINNET_ARB_EVM_TOKENS: Map<Token, EvmTokenConfig> = new Map([
	[
		'ICP',
		{
			mode: 'minter',
			erc20: '0x00f3C42833C3170159af4E92dbb451Fb3F708917'
		}
	],
	[
		'USDC',
		{
			mode: 'locker',
			erc20: '0xaf88d065e77c8cC2239327C5EDb3A432268e5831',
			locker: '0xAe2351B15cFf68b5863c6690dCA58Dce383bf45A'
		}
	],
	[
		'cbBTC',
		{
			mode: 'locker',
			erc20: '0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf',
			locker: '0x7744c6a83E4b43921f27d3c94a742bf9cd24c062'
		}
	],
	[
		'ckBTC',
		{
			mode: 'minter',
			erc20: '0x919A41Ea07c26f0001859Bc5dcb8754068718Fb7'
		}
	],
	[
		'GLDT',
		{
			mode: 'minter',
			erc20: '0x86856814e74456893Cfc8946BedcBb472b5fA856'
		}
	],
	[
		'BOB',
		{
			mode: 'minter',
			erc20: '0xecc5f868AdD75F4ff9FD00bbBDE12C35BA2C9C89'
		}
	],
	[
		'CHAT',
		{
			mode: 'minter',
			erc20: '0xDb95092C454235E7e666c4E226dBBbCdeb499d25'
		}
	]
]);

const MAINNET_ETH_EVM_TOKENS: Map<Token, EvmTokenConfig> = new Map([
	[
		'ICP',
		{
			mode: 'minter',
			erc20: '0x00f3C42833C3170159af4E92dbb451Fb3F708917'
		}
	],
	[
		'USDC',
		{
			mode: 'locker',
			erc20: '0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48',
			locker: '0xAe2351B15cFf68b5863c6690dCA58Dce383bf45A'
		}
	],
	[
		'USDT',
		{
			mode: 'locker',
			erc20: '0xdAC17F958D2ee523a2206206994597C13D831ec7',
			locker: '0xc5AC945a0af0768929301A27D6f2a7770995fAeb'
		}
	],
	[
		'cbBTC',
		{
			mode: 'locker',
			erc20: '0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf',
			locker: '0x7744c6a83E4b43921f27d3c94a742bf9cd24c062'
		}
	],
	[
		'ckBTC',
		{
			mode: 'minter',
			erc20: '0x919A41Ea07c26f0001859Bc5dcb8754068718Fb7'
		}
	],
	[
		'GLDT',
		{
			mode: 'minter',
			erc20: '0x86856814e74456893Cfc8946BedcBb472b5fA856'
		}
	],
	[
		'BOB',
		{
			mode: 'minter',
			erc20: '0xecc5f868AdD75F4ff9FD00bbBDE12C35BA2C9C89'
		}
	],
	[
		'CHAT',
		{
			mode: 'minter',
			erc20: '0xDb95092C454235E7e666c4E226dBBbCdeb499d25'
		}
	]
]);

const LOCAL_ICP_TOKENS: Map<Token, IcpTokenConfig> = new Map([
	[
		'ICP',
		{
			mode: 'locker',
			ledger: Principal.fromText('ryjl3-tyaaa-aaaaa-aaaba-cai'),
			fee: 0.0001
		}
	],
	[
		'USDC',
		{
			mode: 'minter',
			ledger: Principal.fromText('53nhb-haaaa-aaaar-qbn5q-cai'),
			fee: 0.01
		}
	]
]);

const STAGING_ICP_TOKENS: Map<Token, IcpTokenConfig> = new Map([
	[
		'ICP',
		{
			mode: 'locker',
			ledger: Principal.fromText('ryjl3-tyaaa-aaaaa-aaaba-cai'),
			fee: 0.0001
		}
	],
	[
		'USDC',
		{
			mode: 'minter',
			ledger: Principal.fromText('7csws-aiaaa-aaaar-qaqpa-cai'),
			fee: 0.01
		}
	],
	[
		'USDT',
		{
			mode: 'minter',
			ledger: Principal.fromText('n4dku-tiaaa-aaaar-qboqa-cai'),
			fee: 0.01
		}
	],
	[
		'cbBTC',
		{
			mode: 'minter',
			ledger: Principal.fromText('n3cma-6qaaa-aaaar-qboqq-cai'),
			fee: 0.0000002
		}
	],
	[
		'ckBTC',
		{
			mode: 'locker',
			ledger: Principal.fromText('mxzaz-hqaaa-aaaar-qaada-cai'),
			fee: 0.0000001
		}
	],
	[
		'GLDT',
		{
			mode: 'locker',
			ledger: Principal.fromText('6c7su-kiaaa-aaaar-qaira-cai'),
			fee: 0.1
		}
	],
	[
		'BOB',
		{
			mode: 'locker',
			ledger: Principal.fromText('7pail-xaaaa-aaaas-aabmq-cai'),
			fee: 0.01
		}
	],
	[
		'ckUSDC',
		{
			mode: 'unwrapper',
			ledger: Principal.fromText('xevnm-gaaaa-aaaar-qafnq-cai'),
			fee: 0.01
		}
	],
	[
		'ckUSDT',
		{
			mode: 'unwrapper',
			ledger: Principal.fromText('cngnf-vqaaa-aaaar-qag4q-cai'),
			fee: 0.01
		}
	]
]);

export const MAINNET_ICP_TOKENS: Map<Token, IcpTokenConfig> = new Map([
	[
		'ICP',
		{
			mode: 'locker',
			ledger: Principal.fromText('ryjl3-tyaaa-aaaaa-aaaba-cai')
		}
	],
	[
		'USDC',
		{
			mode: 'minter',
			ledger: Principal.fromText('53nhb-haaaa-aaaar-qbn5q-cai'),
			fee: 0.01
		}
	],
	[
		'USDT',
		{
			mode: 'minter',
			ledger: Principal.fromText('ij33n-oiaaa-aaaar-qbooa-cai'),
			fee: 0.01
		}
	],
	[
		'cbBTC',
		{
			mode: 'minter',
			ledger: Principal.fromText('io25z-dqaaa-aaaar-qbooq-cai'),
			fee: 0.0000002
		}
	],
	[
		'GLDT',
		{
			mode: 'locker',
			ledger: Principal.fromText('6c7su-kiaaa-aaaar-qaira-cai'),
			fee: 0.1
		}
	],
	[
		'ckBTC',
		{
			mode: 'locker',
			ledger: Principal.fromText('mxzaz-hqaaa-aaaar-qaada-cai'),
			fee: 0.0000001
		}
	],
	[
		'BOB',
		{
			mode: 'locker',
			ledger: Principal.fromText('7pail-xaaaa-aaaas-aabmq-cai'),
			fee: 0.01
		}
	],
	[
		'CHAT',
		{
			mode: 'locker',
			ledger: Principal.fromText('2ouva-viaaa-aaaaq-aaamq-cai'),
			fee: 0.001
		}
	],
	[
		'ckUSDC',
		{
			mode: 'unwrapper',
			ledger: Principal.fromText('xevnm-gaaaa-aaaar-qafnq-cai'),
			fee: 0.01
		}
	],
	[
		'ckUSDT',
		{
			mode: 'unwrapper',
			ledger: Principal.fromText('cngnf-vqaaa-aaaar-qag4q-cai'),
			fee: 0.01
		}
	]
]);

function getIcpTokens(): Map<Token, IcpTokenConfig> {
	if (DEV) {
		return LOCAL_ICP_TOKENS;
	}
	if (STAGING) {
		return STAGING_ICP_TOKENS;
	}
	// DEV and mainnet have the same token configs.
	return MAINNET_ICP_TOKENS;
}

function getEvmTokens(chain: EvmChain): Map<Token, EvmTokenConfig> {
	if (DEV) return LOCAL_EVM_TOKENS;
	if (STAGING) {
		switch (chain) {
			case 'Base':
				return STAGING_BASE_EVM_TOKENS;
			case 'Arbitrum':
				return STAGING_ARB_EVM_TOKENS;
			case 'Ethereum':
				return STAGING_ETH_EVM_TOKENS;
		}
	} else {
		switch (chain) {
			case 'Base':
				return MAINNET_BASE_EVM_TOKENS;
			case 'Arbitrum':
				return MAINNET_ARB_EVM_TOKENS;
			case 'Ethereum':
				return MAINNET_ETH_EVM_TOKENS;
		}
	}
}

export const CONFIG: Config = {
	icp: {
		oneSec: Principal.fromText(CANISTER_ID_ONE_SEC),
		ckUnwrap: Principal.fromText(CANISTER_ID_CK_UNWRAP),
		logoPath: '/icons/token/icp.webp',
		token: getIcpTokens()
	},
	evm: new Map([
		[
			'Base',
			{
				chainId: DEV ? 31337 : 8453,
				rpcUrl: DEV
					? ['http://localhost:8545']
					: [
							'https://base.llamarpc.com',
							'https://mainnet.base.org',
							'https://base.publicnode.com',
							'https://1rpc.io/base'
						],
				logoPath: '/icons/chain/base.svg',
				safetyMargin: 10,
				blockTimeMs: DEV ? 150 : 1_900,
				token: getEvmTokens('Base')
			}
		],
		[
			'Arbitrum',
			{
				chainId: DEV ? 31338 : 42161,
				rpcUrl: DEV
					? ['http://localhost:8546']
					: ['https://arb1.arbitrum.io/rpc', 'https://arbitrum-one.publicnode.com'],
				logoPath: '/icons/chain/arbitrum.svg',
				safetyMargin: DEV ? 10 : 80,
				blockTimeMs: DEV ? 150 : 240,
				token: getEvmTokens('Arbitrum')
			}
		],
		[
			'Ethereum',
			{
				chainId: DEV ? 31339 : 1,
				rpcUrl: DEV
					? ['http://localhost:8547']
					: ['https://ethereum.publicnode.com', 'https://1rpc.io/eth', 'https://eth.llamarpc.com'],
				logoPath: '/icons/chain/ethereum.svg',
				safetyMargin: DEV ? 1 : 2,
				blockTimeMs: DEV ? 150 : 12_000,
				token: getEvmTokens('Ethereum')
			}
		]
	])
};

console.log(CONFIG);

/// The transfer fee is multiplied by this factor before being presented to the
/// user for confirmation. This is to reduce chances of the transfer failing due
/// to concurrent fee increases.
export const TRANSFER_FEE_MULTIPLIER: Map<Chain, number> = new Map([
	['ICP', 1.0],
	['Base', 1.5],
	['Arbitrum', 1.5],
	['Ethereum', 1.2]
]);

export const TOKEN: Map<Token, TokenConfig> = new Map([
	[
		'ICP',
		{
			decimals: 8,
			logoPath: '/icons/token/icp.webp',
			ledgerFee: 0.0001
		}
	],
	[
		'USDC',
		{
			decimals: 6,
			logoPath: '/icons/token/usdc.svg',
			ledgerFee: 0.01
		}
	],
	[
		'USDT',
		{
			decimals: 6,
			logoPath: '/icons/token/usdt.svg',
			ledgerFee: 0.01
		}
	],
	[
		'cbBTC',
		{
			decimals: 8,
			logoPath: '/icons/token/cbbtc.svg',
			ledgerFee: 0.0000002
		}
	],
	[
		'ckBTC',
		{
			decimals: 8,
			logoPath: '/icons/token/ckbtc.svg',
			ledgerFee: 0.0000001
		}
	],
	[
		'GLDT',
		{
			decimals: 8,
			logoPath: '/icons/token/gldt.svg',
			ledgerFee: 0.1
		}
	],
	[
		'BOB',
		{
			decimals: 8,
			logoPath: '/icons/token/bob.png',
			ledgerFee: 0.01
		}
	],
	[
		'CHAT',
		{
			decimals: 8,
			logoPath: '/icons/token/openchat.svg',
			ledgerFee: 0.001
		}
	],
	[
		'ckUSDC',
		{
			decimals: 6,
			logoPath: '/icons/token/ckusdc.svg',
			ledgerFee: 0.01
		}
	],
	[
		'ckUSDT',
		{
			decimals: 6,
			logoPath: '/icons/token/ckusdt.svg',
			ledgerFee: 0.01
		}
	]
]);

export const ABI = {
	erc20_and_minter: [
		'function balanceOf(address account) view returns (uint256)',
		'function burn1(uint256 amount, bytes32 data1)',
		'function approve(address spender, uint256 amount) returns (bool)'
	],
	erc20: [
		'function balanceOf(address account) view returns (uint256)',
		'function approve(address spender, uint256 amount) returns (bool)'
	],
	locker: ['function lock1(uint256 amount, bytes32 data1)']
};

export const SUPPORTED_ON_ICP: Asset[] = Array.from(
	CONFIG.icp.token
		.keys()
		.map((token) => new Asset('ICP', token))
		.filter((t) => t.token !== 'ckBTC')
);

export const SUPPORTED_ON_EVM: Asset[] = Array.from(
	CONFIG.evm
		.entries()
		.flatMap((x) => x[1].token.keys().map((token) => new Asset(x[0], token)))
		.filter((t) => t.token !== 'ckBTC')
);

export const SUPPORTED: Asset[] = SUPPORTED_ON_ICP.concat(SUPPORTED_ON_EVM);

export function getAvailableChains(token: Token): Chain[] {
	return SUPPORTED.filter((asset) => asset.token === token).map((asset) => asset.chain);
}

export function getAvailableTokens(chain: Chain): Token[] {
	return SUPPORTED.filter((asset) => asset.chain === chain).map((asset) => asset.token);
}

export function bridgeable(src: Asset, dst: Asset): boolean {
	if (
		src.chain === 'ICP' &&
		dst.chain === 'ICP' &&
		src.token === 'ckUSDC' &&
		dst.token === 'USDC'
	) {
		return true;
	}
	if (
		src.chain === 'ICP' &&
		dst.chain === 'ICP' &&
		src.token === 'ckUSDT' &&
		dst.token === 'USDT'
	) {
		return true;
	}
	if (SUPPORTED_ON_EVM.map((asset) => asset.chain).includes(src.chain)) {
		return dst.chain === 'ICP' && src.token === dst.token;
	} else {
		return dst.chain !== 'ICP' && src.token === dst.token;
	}
}

export function tokenLogoPath(token: Token): string {
	return TOKEN.get(token)!.logoPath;
}

export function tokenToDecimals(token: Token): number {
	return TOKEN.get(token)!.decimals;
}

export function tokenToLedgerFee(token: Token): number {
	return TOKEN.get(token)!.ledgerFee;
}

export function chainLogoPath(chain: Chain): string {
	if (chain === 'ICP') {
		return CONFIG.icp.logoPath;
	} else {
		return CONFIG.evm.get(chain)!.logoPath;
	}
}

export function getTxExplorerUrl(chain: Chain, token: Token, tx: string): string {
	switch (chain) {
		case 'Base':
			return `https://basescan.org/tx/${tx}`;
		case 'Arbitrum':
			return `https://arbiscan.io/tx/${tx}`;
		case 'Ethereum':
			return `https://etherscan.io/tx/${tx}`;
		case 'ICP':
			const ledgerId = MAINNET_ICP_TOKENS.get(token)!.ledger;
			return `https://dashboard.internetcomputer.org/tokens/${ledgerId}/transaction/${tx}`;
	}
}

export function getAccountExplorerUrl(chain: Chain, token: Token, account: string): string {
	switch (chain) {
		case 'Base':
			return `https://basescan.org/address/${account}`;
		case 'Arbitrum':
			return `https://arbiscan.io/address/${account}`;
		case 'Ethereum':
			return `https://etherscan.io/address/${account}`;
		case 'ICP':
			const ledgerId = MAINNET_ICP_TOKENS.get(token)!.ledger;
			return `https://dashboard.internetcomputer.org/tokens/${ledgerId}/account/${account}`;
	}
}
