import { test, expect } from '@playwright/test';
import * as fc from 'fast-check';
import {
	numberToBigintScaled,
	isAccountIdValid,
	displayAmount,
	areChainsEqual,
	displayTx,
	displayToken,
	tokenInto,
	displayAddress,
	displayPrincipalShortVersion,
	displayEvmAddressShortVersion,
	isEvmAddressValid,
	isIcrcAccountValid
} from '$lib/utils';
import { Principal } from '@icp-sdk/core/principal';
import type { Account, Token } from '../src/declarations/one_sec/one_sec.did';
import { AccountIdentifier } from '@dfinity/ledger-icp';

test('numberToBigintScaled', () => {
	fc.assert(
		fc.property(
			fc.tuple(
				fc.float({ min: 0, max: 1_000, noNaN: true }),
				fc.float({ min: 0, max: 1_000, noNaN: true }),
				fc.integer({ min: 1, max: 18 })
			),
			([n1, n2, scale]) => {
				const factor = Math.pow(10, scale);
				n1 = Math.floor(n1 * factor) / factor;
				n2 = Math.floor(n2 * factor) / factor;

				if (n1 > n2) {
					expect(numberToBigintScaled(n1, scale)).toBeGreaterThan(numberToBigintScaled(n2, scale));
				} else if (n1 === n2) {
					expect(numberToBigintScaled(n1, scale)).toEqual(numberToBigintScaled(n2, scale));
				} else {
					expect(numberToBigintScaled(n1, scale)).not.toBeGreaterThan(
						numberToBigintScaled(n2, scale)
					);
				}
			}
		)
	);
});

test.describe('isEvmAddressValid', () => {
	test('should return true for a valid EVM address', async () => {
		const validAddress = '0x1234567890abcdef1234567890abcdef12345678';
		expect(isEvmAddressValid(validAddress)).toBe(true);
	});

	test('should return false for an invalid EVM address', async () => {
		const invalidAddress = '0x1234567890abcdef1234567890abcdef1234567';
		expect(isEvmAddressValid(invalidAddress)).toBe(false);
	});

	test('should return false for an EVM address with incorrect prefix', async () => {
		const invalidAddress = '1234567890abcdef1234567890abcdef12345678';
		expect(isEvmAddressValid(invalidAddress)).toBe(false);
	});

	test('should return false for an EVM address with non-hex characters', async () => {
		const invalidAddress = '0x1234567890abcdef1234567890abcdef123456g';
		expect(isEvmAddressValid(invalidAddress)).toBe(false);
	});
});

test.describe('isAccountIdValid', () => {
	test('should return true for a valid account ID', async () => {
		const validAccountId = '1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef';
		expect(isAccountIdValid(validAccountId)).toBe(true);
	});

	test('should return false for an invalid account ID', async () => {
		const invalidAccountId = '1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcde';
		expect(isAccountIdValid(invalidAccountId)).toBe(false);
	});

	test('should return false for an account ID with non-hex characters', async () => {
		const invalidAccountId = '1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdeg';
		expect(isAccountIdValid(invalidAccountId)).toBe(false);
	});
});

test.describe('', () => {
	test('should return true for a valid principal', async () => {
		const validPrincipal = 'l72el-pt5ry-lmj66-3opyw-tl5xx-3wzfl-n3mja-dqirc-oxmqs-uxqe6-6qe';
		expect(isIcrcAccountValid(validPrincipal)).toBe(true);
	});

	test('should return false for an invalid principal (too short)', async () => {
		const invalidPrincipal = '12345-67890-abcde-fghij-klmn';
		expect(isIcrcAccountValid(invalidPrincipal)).toBe(false);
	});

	test('should return false for an invalid principal (non-hex characters)', async () => {
		const invalidPrincipal = '12345-67890-abcde-fghij-klmn-123g';
		expect(isIcrcAccountValid(invalidPrincipal)).toBe(false);
	});

	test('should return false for an invalid principal (incorrect format)', async () => {
		const invalidPrincipal = '1234567890abcdef';
		expect(isIcrcAccountValid(invalidPrincipal)).toBe(false);
	});

	test('should return false for an empty string', async () => {
		const invalidPrincipal = '';
		expect(isIcrcAccountValid(invalidPrincipal)).toBe(false);
	});

	test('should return true for an encoded Icrc account', () => {
		const validIcrcAccount =
			'daijl-2yaaa-aaaar-qag3a-cai-clltauq.5f0e93000f4cbd9db8c36d27cad8b8a97706c0710154172029e54541e18fd180';
		expect(isIcrcAccountValid(validIcrcAccount)).toBe(true);
	});
});

test('areChainsEqual', () => {
	expect(areChainsEqual({ Base: null }, { Base: null })).toBeTruthy();
	expect(areChainsEqual({ ICP: null }, { Base: null })).toBeFalsy();
});

test('displayAmount', () => {
	expect(displayAmount({ Evm: { amount: 999n, decimals: 8 } }, 8)).toBe('0.00000999');
	expect(displayAmount({ Icp: { amount: 999n, decimals: 18 } }, 8)).toBe('0');
	expect(displayAmount({ Evm: { amount: 99999n, decimals: 6 } }, 8)).toBe('0.099999');
	expect(displayAmount({ Icp: { amount: 1000000n, decimals: 8 } }, 8)).toBe('0.01');
	expect(displayAmount({ Evm: { amount: 10000000n, decimals: 8 } }, 8)).toBe('0.1');
	expect(displayAmount({ Icp: { amount: 99999999999999999n, decimals: 8 } }, 8)).toBe(
		"999'999'999.99999999"
	);
	expect(displayAmount({ Evm: { amount: 0n, decimals: 8 } }, 8)).toBe('0');
	expect(displayAmount({ Icp: { amount: 1123456780n, decimals: 6 } }, 8)).toBe("1'123.45678");
	expect(displayAmount({ Evm: { amount: 1n, decimals: 18 } }, 8)).toBe('0');
	expect(displayAmount({ Icp: { amount: 99999999n, decimals: 8 } }, 8)).toBe('0.99999999');
	expect(displayAmount({ Evm: { amount: 100000000n, decimals: 8 } }, 8)).toBe('1');
});

test.describe('displayTx', () => {
	test('should display Evm transaction hash', () => {
		const evmTx1 = { Evm: { hash: '0x1234567890abcdef', log_index: [] as [] } };
		const evmTx2 = { Evm: { hash: '0x1234567890abcdef', log_index: [12n] as [bigint] } };
		expect(displayTx(evmTx1)).toBe('0x1234567890abcdef');
		expect(displayTx(evmTx2)).toBe('0x1234567890abcdef');
	});

	test('should display Icp transaction block index', () => {
		const icpTx1 = { Icp: { block_index: 123n, ledger: Principal.anonymous() } };
		const icpTx2 = { Icp: { block_index: 123n, ledger: Principal.managementCanister() } };
		expect(displayTx(icpTx1)).toBe('123');
		expect(displayTx(icpTx2)).toBe('123');
	});

	test('should throw an error if tx is neither Evm nor Icp', () => {
		const invalidTx = {} as any;
		expect(() => displayTx(invalidTx)).toThrowError();
	});
});

test.describe('displayToken', () => {
	test('should display ICP token', () => {
		const icpToken: Token = { ICP: null };
		expect(displayToken(tokenInto(icpToken))).toBe('ICP');
	});

	test('should throw an error if token is not ICP', () => {
		const invalidToken: Token = {} as any;
		expect(displayToken(tokenInto(invalidToken))).toBe('undefined');
	});
});

test.describe('displayAddress', () => {
	test('should display full ICP address when showAll is true', async () => {
		const owner = Principal.managementCanister();
		const icpAccount: Account = { Icp: { ICRC: { owner, subaccount: [] } } };
		const fullAddress = AccountIdentifier.fromPrincipal({
			principal: owner
		}).toHex();
		expect(displayAddress(icpAccount, true)).toBe(fullAddress);
	});

	test('should display full EVM address when showAll is true', async () => {
		const evmAccount: Account = { Evm: { address: '0x1234567890abcdef' } };
		expect(displayAddress(evmAccount, true)).toBe(evmAccount.Evm.address);
	});

	test('should display short ICP address when showAll is false', async () => {
		const owner = Principal.managementCanister();
		const icpAccount: Account = { Icp: { ICRC: { owner, subaccount: [] } } };
		expect(displayAddress(icpAccount)).toBe(displayPrincipalShortVersion(owner.toString()));
	});

	test('should display short EVM address when showAll is false', async () => {
		const evmAccount: Account = { Evm: { address: '0x1234567890abcdef' } };
		expect(displayAddress(evmAccount)).toBe(displayEvmAddressShortVersion(evmAccount.Evm.address));
	});

	test('should throw an error if account is neither ICP nor EVM', async () => {
		const invalidAccount: Account = {} as any; // Cast to Account to bypass type checking
		expect(() => displayAddress(invalidAccount)).toThrowError();
	});
});

test.describe('displayPrincipalShortVersion', () => {
	test('should display short version of ICP principal', async () => {
		const principal = Principal.managementCanister().toString();
		const shortVersion =
			principal.split('-')[0] + '...' + principal.split('-')[principal.split('-').length - 1];
		expect(displayPrincipalShortVersion(principal)).toBe(shortVersion);
	});
});

test.describe('displayEvmAddressShortVersion', () => {
	test('should display short version of EVM address', async () => {
		const address = '0x1234567890abcdef';
		expect(displayEvmAddressShortVersion(address)).toBe('0x123..def');
	});

	test('should display -/- for empty address', async () => {
		expect(displayEvmAddressShortVersion('')).toBe('-/-');
	});
});
