import type { Chain, Token } from './types';

export const NANOS_PER_MS: number = 1_000_000;
export const MS_PER_SECOND: number = 1_000;
export const MS_PER_HOUR: number = 3_600 * MS_PER_SECOND;
export const MS_PER_DAY: number = 24 * MS_PER_HOUR;
export const MS_PER_WEEK: number = 7 * MS_PER_DAY;
export const ICP_CALL_MS: number = 6 * MS_PER_SECOND;

export function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

export function txError(err: Error): string {
	if (err.message.includes('reverted')) {
		return 'transaction reverted';
	}
	return 'cancelled by user';
}

export function sortedChains(cs: Chain[]): Chain[] {
	const result: Chain[] = [...cs];
	result.sort((a, b) => (a < b ? -1 : 1));
	return result;
}

export async function fetchPriceCoinGecko(): Promise<Map<Token, number>> {
	const tokenNames = [
		'usd-coin',
		'internet-computer',
		'coinbase-wrapped-btc',
		'tether',
		'chain-key-bitcoin',
		'bob-3',
		'gold-token',
		'openchat'
	];
	const options = {
		method: 'GET',
		headers: {
			accept: 'application/json'
		}
	};

	const tokenIds = tokenNames.join(',');

	try {
		const response = await fetch(
			`https://api.coingecko.com/api/v3/simple/price?ids=${tokenIds}&vs_currencies=usd`,
			options
		);

		if (!response.ok) {
			throw new Error(`HTTP error! status: ${response.status}`);
		}

		const data = await response.json();
		console.log(data);
		return new Map([
			['ICP', Number(data['internet-computer']['usd'])],
			['USDC', Number(data['usd-coin']['usd'])],
			['ckUSDC', Number(data['usd-coin']['usd'])],
			['USDT', Number(data['tether']['usd'])],
			['ckUSDT', Number(data['tether']['usd'])],
			['cbBTC', Number(data['coinbase-wrapped-btc']['usd'])],
			['ckBTC', Number(data['chain-key-bitcoin']['usd'])],
			['BOB', Number(data['bob-3']['usd'])],
			['GLDT', Number(data['gold-token']['usd'])],
			['CHAT', Number(data['openchat']['usd'])]
		]);
	} catch (err) {
		console.error('Error fetching prices from CoinGecko:', err);
		return new Map();
	}
}
