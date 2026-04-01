export const BALANCE_UPDATE_MS: number = 10_000;

export interface Balance {
	amount: number;
	lastUpdated: Date;
}

export type Amount = {
	value: bigint;
	decimals: number;
};
