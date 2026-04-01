import type { Account, IcpAccount, Tx } from '../declarations/one_sec/one_sec.did';
import { decodeIcrcAccount } from '@dfinity/ledger-icrc';
import { Principal } from '@dfinity/principal';
import type { Amount } from '$lib/types';
import { IcpUser } from './user/icpUser';
import { EvmUser } from './user/evmUser';
import { ethers } from 'ethers';
import { AccountIdentifier } from '@dfinity/ledger-icp';
import type { Chain, EvmChain, Token as TokenEnum } from './oneSec/types';
import { bigEndianCrc32, uint8ArrayToHexString } from '@dfinity/utils';

export function unpackAmount(amount: Amount): [number, bigint] {
	return [amount.decimals, amount.value];
}

export function displayTimeSeconds(time: number): string {
	if (time < 60) {
		return `${time}s`;
	} else if (time < 3600) {
		return `${(time / 60).toFixed(0)}m`;
	} else if (time < 86400) {
		return `${(time / 3600).toFixed(0)}h`;
	} else {
		return `${(time / 86400).toFixed(0)}d`;
	}
}

export function displayAgeTimestampSeconds(ts: number): string {
	const elapsed = (Date.now() - ts) / 1_000;
	if (elapsed < 60) {
		return `${elapsed.toFixed(0)} seconds ago`;
	} else if (elapsed < 3_600) {
		return `${(elapsed / 60).toFixed(0)} minute${elapsed >= 2 * 60 ? 's' : ''} ago`;
	} else if (elapsed < 86_400) {
		return `${(elapsed / 3_600).toFixed(0)} hour${elapsed >= 2 * 3_600 ? 's' : ''} ago`;
	} else if (elapsed < 2_592_000) {
		return `${(elapsed / 86_400).toFixed(0)} day${elapsed >= 2 * 86_400 ? 's' : ''} ago`;
	} else if (elapsed < 31_536_000) {
		return `${(elapsed / 2_592_000).toFixed(0)} month${elapsed >= 2 * 2_592_000 ? 's' : ''} ago`;
	} else {
		return `${(elapsed / 31_536_000).toFixed(0)} year${elapsed >= 2 * 31_536_000 ? 's' : ''} ago`;
	}
}

export function displayTxWithInfo(
	txUrl: string,
	info: { address: string } | { txHash: string }
): string {
	if ('address' in info) return txUrl + info.address;
	if ('txHash' in info) return txUrl + info.txHash;
	throw new Error('Unknown configuration to display.');
}

export function displayTx(tx: Tx, showAll = true): string {
	if ('Evm' in tx) {
		return showAll ? tx.Evm.hash : truncateAddress(tx.Evm.hash);
	} else {
		return tx.Icp.block_index.toString();
	}
}

export function displayAddress(a?: Account, showAll = false): string {
	if (!a) {
		return 'N/A';
	}
	if (showAll) {
		if ('Icp' in a) {
			if ('ICRC' in a.Icp) {
				return a.Icp.ICRC.owner.toString();
			} else if ('AccountId' in a.Icp) {
				return a.Icp.AccountId;
			} else {
				return '';
			}
		} else {
			return a.Evm.address;
		}
	} else {
		if ('Icp' in a) {
			if ('ICRC' in a.Icp) {
				return truncateAddress(a.Icp.ICRC.owner.toString());
			} else if ('AccountId' in a.Icp) {
				return truncateAddress(a.Icp.AccountId);
			} else {
				return '';
			}
		} else {
			return truncateAddress(a.Evm.address);
		}
	}
}

function splitAddress(str: string, chars: number) {
	const half = Math.floor(chars / 2);
	let first = str.slice(0, half);
	let last = str.slice(str.length - half);

	if (first.endsWith('-')) {
		first = first.slice(0, -1);
	}
	return { first, last };
}

export function truncateAddress(address: string, charsToDisplay?: number) {
	if (address.includes('-')) {
		if (charsToDisplay) {
			const { first, last } = splitAddress(address, charsToDisplay);
			return `${first}..${last}`;
		} else {
			const parts = address.split('-');
			return `${parts[0]}-...-${parts[parts.length - 1]}`;
		}
	} else {
		if (charsToDisplay) {
			const { first, last } = splitAddress(address, charsToDisplay);
			return `${first}..${last}`;
		} else {
			return `${address.slice(0, 5)}...${address.slice(-3)}`;
		}
	}
}

export function isEvmAddressValid(address: string): boolean {
	const regex = /^0x[0-9a-fA-F]{40}$/;
	return regex.test(address);
}

export function isAccountIdValid(address: string): boolean {
	const regex = /^[0-9a-fA-F]{64}$/;
	return regex.test(address);
}

export function isIcrcAccountValid(address: string): boolean {
	try {
		decodeIcrcAccount(address);
		return true;
	} catch (_) {
		return false;
	}
}

export function handleRawInput(rawInput: string): string {
	const regex = /[0-9]/;
	let isDecimalPart = false;
	let res = '';
	for (const char of rawInput) {
		if (regex.test(char)) {
			res += char;
		} else if (char === '.' && !isDecimalPart) {
			res += '.';
			isDecimalPart = true;
		}
	}
	return res;
}

export function displayValue(value: number, maxDecimals?: number) {
	let decimalsToDisplay = 0;
	if (value > 100) {
		decimalsToDisplay = 2;
	} else if (value > 1) {
		decimalsToDisplay = 4;
	} else if (value > 0.01) {
		decimalsToDisplay = 6;
	} else if (value > 0.0001) {
		decimalsToDisplay = 8;
	} else if (value > 0.000001) {
		decimalsToDisplay = 10;
	}

	if (maxDecimals) decimalsToDisplay = Math.min(maxDecimals, decimalsToDisplay);

	if (value < 0) {
		return (
			'-' +
			displayNumber({
				value: numberToBigintScaled(Math.abs(value), 8),
				decimals: 8,
				decimalsToDisplay
			})
		);
	} else {
		return displayNumber({
			value: numberToBigintScaled(value, 8),
			decimals: 8,
			decimalsToDisplay
		});
	}
}

export function displayNumber({
	value,
	decimals,
	decimalsToDisplay
}: {
	value: bigint;
	decimals: number;
	decimalsToDisplay: number;
}): string {
	const padded = value.toString().padStart(decimals, '0');
	const [integer, fraction] = [
		padded.slice(0, -decimals).replace(/^0+/, '') || '0',
		padded.slice(-decimals).replace(/0+$/, '')
	];

	const formattedInteger = integer.replace(/\B(?=(\d{3})+(?!\d))/g, "'");
	const formattedFraction = fraction.slice(0, decimalsToDisplay).replace(/0+$/, '');

	return formattedFraction ? `${formattedInteger}.${formattedFraction}` : formattedInteger;
}

export function numberToBigintScaled(value: number, decimals: number): bigint {
	const [integerPart, fractionalPart = ''] = value.toFixed(decimals).split('.');

	const paddedFractionalPart = fractionalPart.padEnd(decimals, '0').slice(0, decimals);

	const combined = `${integerPart}${paddedFractionalPart}`;
	return BigInt(combined);
}

export const isMobile = typeof window !== 'undefined' && window.innerWidth <= 767;

export function accountIdentifierFromHex(hex: string): AccountIdentifier {
	const buffer = Uint8Array.from(Buffer.from(hex, 'hex'));
	const hash = buffer.slice(4);
	const expectedChecksum = uint8ArrayToHexString(buffer.slice(0, 4));
	const actualChecksum = uint8ArrayToHexString(bigEndianCrc32(hash));

	if (expectedChecksum != actualChecksum) {
		throw Error(`invalid account identifier: the check sum does not match`);
	}
	return AccountIdentifier.fromHex(hex);
}

const TAG_ICRC = 0;
const TAG_ACCOUNT_ID = 1;

// Format:
// - bytes[0] = tag: ICRC or account identifier.
// - bytes[1..32] = encoded account.
export function encodeIcpAccount(account: IcpAccount): Uint8Array {
	if ('ICRC' in account) {
		return encodePrincipal(account.ICRC.owner);
	} else if ('AccountId' in account) {
		return encodeAccountId(accountIdentifierFromHex(account.AccountId));
	} else {
		const _unreachable: never = account;
		throw new Error('unreachable');
	}
}

// Format:
// - bytes[0] = 0 (ICRC account tag)
// - bytes[1] = the length of the principal in bytes.
// - bytes[2..length+2] = the principal itself.
// - bytes[length+2..32] = zeros.
export function encodePrincipal(p: Principal): Uint8Array {
	const principal = p.toUint8Array();
	const array = new Uint8Array(32);
	array[0] = TAG_ICRC;
	array[1] = principal.length;
	array.set(principal, 2);
	return array;
}

// Format:
// - bytes[0] = 1  (account identifier tag)
// - bytes[1..29] = the 28 bytes of the account identifier (without the CRC32 checksum).
// - bytes[29..32] = zeros.
export function encodeAccountId(accountId: AccountIdentifier): Uint8Array {
	// Skip the first 4 bytes that correspond to CRC32 checksum.
	const bytes = accountId.toUint8Array().slice(4);
	const array = new Uint8Array(32);
	array[0] = TAG_ACCOUNT_ID;
	array.set(bytes, 1);
	return array;
}

export async function updateBalances(icpUser?: IcpUser, evmUser?: EvmUser) {
	const balances = [];
	if (icpUser) {
		for (let token of icpUser.tokens()) {
			balances.push(icpUser.fetchBalance(token));
		}
	}
	if (evmUser) {
		for (let asset of evmUser.assets()) {
			balances.push(evmUser.fetchBalance(asset.chain as EvmChain, asset.token));
		}
	}
	await Promise.all(balances);
}

export function validateAddrError(address: string, chain?: Chain, token?: TokenEnum): string {
	const result = validateAddr(address, chain, token);
	return typeof result === 'string' ? result : '';
}

export function validateAddr(address: string, chain?: Chain, token?: TokenEnum): Account | string {
	if (!chain) {
		if (address.startsWith('0x') && address.length == 42) {
			try {
				ethers.getAddress(address);
				return { Evm: { address } };
			} catch (err) {
				return 'Invalid EVM address';
			}
		}

		try {
			const principal = Principal.fromText(address);
			return { Icp: { ICRC: { owner: principal, subaccount: [] } } };
		} catch {
			// Fall through to account identifier.
		}

		if (!token || token === 'ICP') {
			try {
				accountIdentifierFromHex(address);
				return { Icp: { AccountId: address } };
			} catch {
				// Fall through to the error.
			}
		}

		return 'Invalid address';
	}

	switch (chain) {
		case 'Arbitrum':
		case 'Base':
		case 'Ethereum': {
			try {
				ethers.getAddress(address);
				return { Evm: { address } };
			} catch (err) {
				return 'Invalid EVM address';
			}
		}
		case 'ICP': {
			try {
				const principal = Principal.fromText(address);
				return { Icp: { ICRC: { owner: principal, subaccount: [] } } };
			} catch {
				// Fall through to account identifier.
			}

			if (!token || token === 'ICP') {
				try {
					accountIdentifierFromHex(address);
					return { Icp: { AccountId: address } };
				} catch {
					return 'Please enter a valid principal or account-id';
				}
			} else {
				return 'Please enter a valid principal';
			}
		}
	}
}

export type Size = 'tiny' | 'small' | 'normal' | 'large' | 'huge';

export const MAX_TRANSACTIONS_PER_PAGE = 7;
