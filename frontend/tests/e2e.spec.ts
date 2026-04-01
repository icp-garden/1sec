import { expect, type BrowserContext } from '@playwright/test';
import { testWithII } from '@dfinity/internet-identity-playwright';
import {
	mockSetup,
	isToastSuccess,
	connectWithII,
	supplyAccount,
	bridge,
	connectWithMetamask,
	transfer
} from './utils';
import dappwright, { type Dappwright, MetaMaskWallet } from '@tenkeylabs/dappwright';

const VALID_PRINCIPAL = 'l72el-pt5ry-lmj66-3opyw-tl5xx-3wzfl-n3mja-dqirc-oxmqs-uxqe6-6qe';
const VALID_ACCOUNT_ID = 'e73a99617af2a8dbfe9b75e463e83a905e30aa50250972ad19c21922c22b2a2a';
const VALID_ACCOUNT =
	'daijl-2yaaa-aaaar-qag3a-cai-clltauq.5f0e93000f4cbd9db8c36d27cad8b8a97706c0710154172029e54541e18fd180';
const VALID_EVM_ADDRESS = '0x1234567890abcdef1234567890abcdef12345678';

const test = testWithII.extend<{
	context: BrowserContext;
	wallet: Dappwright;
}>({
	context: async ({}, use) => {
		// Launch context with extension
		const [wallet, _, context] = await dappwright.bootstrap('', {
			wallet: 'metamask',
			version: MetaMaskWallet.recommendedVersion,
			seed: 'test test test test test test test test test test test junk', // Hardhat's default https://hardhat.org/hardhat-network/docs/reference#accounts
			headless: false
		});

		// Add Hardhat as a custom network
		await wallet.addNetwork({
			networkName: 'Local Network',
			rpc: 'http://localhost:8545',
			chainId: 31337,
			symbol: 'ETH'
		});

		await use(context);
	},

	wallet: async ({ context }, use) => {
		const metamask = await dappwright.getWallet('metamask', context);
		await use(metamask);
	}
});

test('Intermediary account should have balance', async () => {
	const { mockCanisters, mockMintingAccount } = await mockSetup();

	if (!(mockCanisters && mockMintingAccount))
		throw new Error('Mock user or mock canisters are undefined.');
	if ('Icp' in mockMintingAccount.account) {
		const icpBalance = await mockCanisters.icpLedger.authenticatedActor?.icrc1_balance_of(
			mockMintingAccount.account.Icp
		);
		console.log('ICP balance of mock minting account:', icpBalance);
		expect(icpBalance && icpBalance > 0n).toBeTruthy();
	}
});

test.describe('test UX with II connect', () => {
	test.beforeEach(async ({ page, iiPage }) => {
		await connectWithII(page, iiPage);
		await supplyAccount(page);
	});

	test('e2e test send', async ({ page }) => {
		const icpBalance = page.locator('#wallet-info').locator('[title="icp-balance-nav"]');
		await expect(icpBalance).toHaveText('15 ICP');

		await page.locator('[title="transfer-btn-ICP"]').click();
		await transfer(page, 'aaa-aa', '10');
		await expect(page.locator('[title="transfer-input-error"]')).toBeVisible();
		expect(await isToastSuccess(page)).toBeFalsy();
		await transfer(page, VALID_ACCOUNT_ID, '16');
		await expect(page.locator('[title="transfer-amount-error"]')).toBeVisible();
		expect(await isToastSuccess(page)).toBeFalsy();
		await transfer(page, VALID_ACCOUNT_ID, '0.00009');
		await expect(page.locator('[title="transfer-amount-error"]')).toBeVisible();
		expect(await isToastSuccess(page)).toBeFalsy();

		await transfer(page, VALID_ACCOUNT_ID, '1');
		expect(await isToastSuccess(page)).toBeTruthy();

		await transfer(page, VALID_ACCOUNT, '1');
		expect(await isToastSuccess(page)).toBeTruthy();

		await page.locator('[title="max-placeholder"]').click();
		const maxAmountSendIcp = parseFloat(
			(await page
				.locator('[title="transfer-input-amount"]')
				.evaluate((input) => (input as HTMLInputElement).value)) ?? '0'
		);
		await transfer(page, VALID_PRINCIPAL, maxAmountSendIcp.toString());
		expect(await isToastSuccess(page)).toBeTruthy();
		await expect(icpBalance).toHaveText('0 ICP');
	});

	test('e2e test deposit', async ({ page }) => {
		const icpBalance = page.locator('#wallet-info').locator('[title="icp-balance-nav"]');
		await expect(icpBalance).toHaveText('15 ICP', { timeout: 10_000 });

		await page.locator('[title="home-btn"]').click();
		await page.locator('[title="source-selection-btn"]').click();
		await page.locator('[title="chain-btn-ICP"]').click();
		await page.locator('[title="token-btn-ICP"]').click();
		await page.locator('[title="destination-selection-btn"]').click();
		await page.locator('[title="chain-btn-Base"]').click();
		await page.locator('[title="token-btn-ggICP"]').click();

		bridge(page, VALID_EVM_ADDRESS, '15');
		await expect(page.locator('[title="bridge-amount-error"]')).toBeVisible();
		bridge(page, VALID_EVM_ADDRESS, '0.009');
		await expect(page.locator('[title="bridge-amount-error"]')).toBeVisible();

		bridge(page, '0x11111a', '15');
		await expect(page.locator('[title="bridge-input-error"]')).toBeVisible();

		await bridge(page, VALID_EVM_ADDRESS, '1');
		await expect(icpBalance).toHaveText('13.99 ICP');

		expect(page.locator('[title="source-amount-status"]')).toHaveText('1 ICP');
		const blockIndex = await page.locator('[title="source-contract-status"]').getAttribute('href');
		expect(blockIndex).toContain('https://dashboard.internetcomputer.org/transaction/');
		const contract = await page
			.locator('[title="destination-contract-status"]')
			.getAttribute('href');
		expect(contract).toContain('https://basescan.org/tx/');
	});
});

test.describe('test UX with EVM connect', () => {
	test.beforeEach(async ({ page, wallet }) => {
		await connectWithMetamask(page, wallet);
	});

	test('e2e test withdraw', async ({ page, wallet }) => {
		const balance = page.locator('#wallet-info').locator('[title="icp-balance-nav"]');
		await expect(balance).toBeVisible();

		await page.locator('[title="source-selection-btn"]').click();
		await page.locator('[title="token-btn-USDC"]').click();
		await page.locator('[title="chain-btn-Base"]').click();
		await page.locator('[title="destination-selection-btn"]').click();
		await page.locator('[title="chain-btn-ICP"]').click();
		await page.locator('[title="token-btn-ggUSDC"]').click();

		bridge(page, VALID_PRINCIPAL, '10000000000000');
		await expect(page.locator('[title="bridge-amount-error"]')).toBeVisible();

		bridge(page, VALID_PRINCIPAL, '0.009');
		await expect(page.locator('[title="bridge-amount-error"]')).toBeVisible();

		bridge(page, '0x11111a', '15');
		await expect(page.locator('[title="bridge-input-error"]')).toBeVisible();

		await bridge(page, VALID_PRINCIPAL, '1');
		await wallet.confirmTransaction();
		await wallet.confirmTransaction();

		const contract = await page.locator('[title="source-contract-status"]').getAttribute('href');
		expect(contract).toContain('https://basescan.org/tx/');
		const blockIndex = await page
			.locator('[title="destination-contract-status"]')
			.getAttribute('href');
		expect(blockIndex).toContain('https://dashboard.internetcomputer.org/transaction/');
	});

	test.only('e2e test send', async ({ page, wallet }) => {
		const icpBalance = page.locator('#wallet-info').locator('[title="icp-balance-nav"]');
		await expect(icpBalance).toBeVisible();

		await page.locator('#wallet-info').click();
		await page.locator('[title="transfer-btn-USDC"]').click();
		await transfer(page, 'aaa-aa', '10');
		await expect(page.locator('[title="transfer-input-error"]')).toBeVisible();
		expect(await isToastSuccess(page)).toBeFalsy();
		await transfer(page, VALID_EVM_ADDRESS, '100000000000');
		await expect(page.locator('[title="transfer-amount-error"]')).toBeVisible();
		expect(await isToastSuccess(page)).toBeFalsy();

		await transfer(page, VALID_EVM_ADDRESS, '1');
		await wallet.confirmTransaction();
		expect(await isToastSuccess(page)).toBeTruthy();
		await page.waitForTimeout(5_000);

		await page.locator('[title="max-placeholder"]').click();
		const maxAmountSendIcp = parseFloat(
			(await page
				.locator('[title="transfer-input-amount"]')
				.evaluate((input) => (input as HTMLInputElement).value)) ?? '0'
		);
		await transfer(page, VALID_EVM_ADDRESS, maxAmountSendIcp.toString());
		await wallet.confirmTransaction();
		expect(await isToastSuccess(page)).toBeTruthy();
	});
});
