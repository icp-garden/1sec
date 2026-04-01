import { HttpAgent } from '@dfinity/agent';
import { Ed25519KeyIdentity } from '@dfinity/identity';
import { fetchActors, User } from '$lib/actors';
import { accountIdentifierFromHex, isAccountIdValid, isIcrcAccountValid } from '$lib/utils';
import type {
	TransferArgs,
	Tokens,
	TransferArg
} from '../src/declarations/icp_ledger/icp_ledger.did';
import { AccountIdentifier } from '@dfinity/ledger-icp';
import type { Page } from 'playwright';
import { expect } from '@playwright/test';
import { Principal } from '@dfinity/principal';
import { InternetIdentityPage } from '@dfinity/internet-identity-playwright';
import type { Dappwright } from '@tenkeylabs/dappwright';
import { ethers } from 'ethers';

const key = [
	'302a300506032b657003210093d488f46b485c07e09b554d9451574bfc669912b99d453722c474e6a7f90fcc',
	'90252a6913658dbb4b36b276410216d47a1891280493cd485328279a12a53e2c'
];

const parsedKey = JSON.stringify(key);

// The key is used to generate an intermediary account dispatching ICP/nICP tokens to testing accounts.
// AccountId = 90526bdfd692793cba1f96bde9079994ce4d40033746f04c12064ea599e2c274
// Principal = syna7-6ipnd-myx4g-ia46u-nxwok-u5nrr-yxgpi-iang7-lvru2-i7n23-tqe

export const mockSetup = async () => {
	const dummyIdentity = Ed25519KeyIdentity.fromJSON(parsedKey);

	const agent = HttpAgent.createSync({ host: 'http://127.0.1:8080', identity: dummyIdentity });
	agent.fetchRootKey().catch((err) => {
		console.warn('Unable to fetch root key. Check to ensure that your local replica is running');
		console.error(err);
	});

	return {
		mockCanisters: await fetchActors(agent),
		mockMintingAccount: new User({ Icp: { owner: dummyIdentity.getPrincipal(), subaccount: [] } })
	};
};

export async function transferICP(accountString: string) {
	const { mockCanisters, mockMintingAccount } = await mockSetup();

	if (!(mockCanisters && mockMintingAccount))
		throw new Error('Mock user or mock canisters are undefined.');

	if (isAccountIdValid(accountString)) {
		const result = await mockCanisters.icpLedger.authenticatedActor?.transfer({
			to: accountIdentifierFromHex(accountString).toUint8Array(),
			fee: { e8s: 10000n } as Tokens,
			memo: 0n,
			from_subaccount: [],
			created_at_time: [],
			amount: { e8s: 1_500_000_000n } as Tokens
		} as TransferArgs);

		if (!result || Object.keys(result)[0] === 'Err') throw new Error('Failed to transfer balance');
	} else if (isIcrcAccountValid(accountString)) {
		const result = await mockCanisters.icpLedger.authenticatedActor?.icrc1_transfer({
			to: { owner: Principal.fromText(accountString), subaccount: [] },
			fee: [],
			memo: [],
			from_subaccount: [],
			created_at_time: [],
			amount: 1_500_000_000n
		} as TransferArg);

		if (!result || Object.keys(result)[0] === 'Err') throw new Error('Failed to transfer balance');
	}
}

export async function supplyGasForAddress() {
	const targetAddress = '0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266';

	const provider = new ethers.JsonRpcProvider('http://localhost:8545');

	const weiAmount = ethers.parseUnits('1', 'ether').toString();

	await provider.send('anvil_setBalance', [targetAddress, '0x' + BigInt(weiAmount).toString(16)]);

	console.log(`Funded ${1} ETH to ${targetAddress}`);
}

export async function isToastSuccess(page: Page) {
	await page.waitForTimeout(2000);
	const message = await page.locator('p[title="toast-message"]').evaluate((msg) => msg.textContent);
	console.log(message);
	await page.locator('.toast-close').click();
	await expect(page.locator('p[title="toast-message"]')).not.toBeVisible();
	return message?.split(' ')[0].slice(0, 7) === 'Success';
}

export async function bridge(page: Page, destination: string, amount: string) {
	await page.locator('[title="bridge-input-destination"]').fill(destination);
	await page.locator('[title="bridge-input-amount"]').fill(amount);

	await page.locator('[title="bridge-btn"]').click();
}

export async function transfer(page: Page, destination: string, amount: string) {
	await page.locator('[title="transfer-input-destination"]').fill(destination);
	await page.locator('[title="transfer-input-amount"]').fill(amount);

	await page.locator('[title="continue-btn"]').click();
}

export async function connectWithII(page: Page, iiPage: InternetIdentityPage) {
	await page.goto('/');

	await page.locator('[title="connect-btn"]').click();

	await iiPage.signInWithNewIdentity({ selector: '[title="ii-connect-btn"]', captcha: true });
}

export async function connectWithMetamask(page: Page, wallet: Dappwright) {
	await page.goto('/');

	await page.locator('[title="connect-btn"]').click();
	await page.locator('[title="MetaMask-connect-btn"]').click();
	await wallet.approve();
}

export async function supplyAccount(page: Page) {
	const walletInfo = page.locator('#wallet-info');
	await expect(walletInfo).toBeVisible();

	await walletInfo.click();

	const accountId = await page.locator('p[title="address-user"]').textContent();

	if (!accountId) throw new Error('No account id found.');

	const principal = await page.locator('p[title="principal-user"]').textContent();

	if (!principal) throw new Error('No principal found.');

	await transferICP(accountId);
}
