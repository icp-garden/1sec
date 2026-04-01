<script lang="ts">
	import { displayAddress, isMobile, validateAddr } from '$lib/utils';
	import type { Account, Transfer, Status } from '$lib/../../declarations/one_sec/one_sec.did';
	import { icpAnonymous } from '$lib/user/icpUser';
	import { TOKEN } from '$lib/oneSec/config';
	import type { Token, Chain, TransferInfo } from '$lib/oneSec/types';
	import SearchInput from './SearchInput.svelte';
	import DefaultHistory from './DefaultHistory.svelte';
	import MobileHistory from './MobileHistory.svelte';

	export let address: string | undefined;
	let totalPages = 0;
	let currentPage = 1;
	let pagesRange: number[] = [];

	let transfers: TransferInfo[] = [];
	let transfersToDisplay: TransferInfo[] = [];
	let isLoading = false;
	const MAX_TRANSACTIONS_PER_PAGE = 7;
	const MAX_PAGES_PER_RANGE = 3;

	function intoTransferInfo(result: Array<Transfer>): TransferInfo[] {
		return result.map((transfer) => {
			const unknown = {
				Unknown: null
			};
			const sourceToken = Object.keys(transfer.source.token[0] ?? unknown)[0] as Token;
			const destinationToken = Object.keys(transfer.destination.token[0] ?? unknown)[0] as Token;
			const sourceChain = Object.keys(transfer.source.chain[0] ?? unknown)[0] as Chain;
			const destinationChain = Object.keys(transfer.destination.chain[0] ?? unknown)[0] as Chain;
			const status = (transfer.status[0] ?? unknown) as Status;

			const deposited =
				Number(transfer.source.amount) / Math.pow(10, TOKEN.get(sourceToken)!.decimals);

			const received =
				Number(transfer.destination.amount) / Math.pow(10, TOKEN.get(destinationToken)!.decimals);

			let ago = transfer.start[0];
			if (!ago && transfer.trace.entries.length > 0) {
				ago = transfer.trace.entries.reduce((acc, curr) =>
					curr.start > acc.start ? curr : acc
				).start;
			} else {
				ago = transfer.end[0];
			}
			return {
				toAddress: displayAddress(transfer.destination.account[0], true),
				fromAddress: displayAddress(transfer.source.account[0], true),
				destinationToken,
				sourceToken,
				deposited,
				received,
				ts_ms: ago,
				tx: transfer.destination.tx,
				status,
				sourceChain,
				destinationChain
			};
		});
	}

	const fetchTransfers = async () => {
		isLoading = true;
		try {
			let accounts: Account[] = [];
			if (address) {
				let account = validateAddr(address);
				if (typeof account !== 'string') {
					accounts.push(account);
				}
			}
			const result = await icpAnonymous().oneSec().get_transfers({
				accounts,
				count: 100_000n,
				skip: 0n
			});
			switch (true) {
				case 'Ok' in result:
					transfers = intoTransferInfo(result.Ok);
					const quotient = Math.floor(result.Ok.length / MAX_TRANSACTIONS_PER_PAGE);
					const remainder = result.Ok.length % MAX_TRANSACTIONS_PER_PAGE;
					totalPages = quotient + (remainder > 0 ? 1 : 0);
					break;
				case 'Err' in result:
					console.log(result.Err);
					break;
			}
		} catch (e) {
			console.log(e);
		}
		isLoading = false;
	};

	async function setTransfersToDisplay() {
		const skip = (currentPage - 1) * MAX_TRANSACTIONS_PER_PAGE;
		transfersToDisplay = transfers.filter((_transfer, index) => {
			return index >= skip && index < skip + MAX_TRANSACTIONS_PER_PAGE;
		});
	}

	$: (currentPage, setTransfersToDisplay());
	$: (address,
		fetchTransfers().then(() => {
			currentPage = 1;
			setTransfersToDisplay();
			setPagesRange();
		}));

	const setPagesRange = () => {
		if (totalPages <= 3) {
			pagesRange = Array.from({ length: totalPages }, (_, i) => i + 1);
		} else if (currentPage === 1) {
			pagesRange = [1, 2, 3];
		} else if (currentPage === totalPages) {
			pagesRange = [totalPages - 2, totalPages - 1, totalPages];
		} else {
			const maxPage = Math.min(currentPage + 1, totalPages);
			const minPage = Math.max(currentPage - 1, 1);
			pagesRange = [];
			for (let i = 0; i <= maxPage - minPage; i++) {
				pagesRange.push(minPage + i);
			}
		}
	};

	$: (currentPage, setPagesRange());
</script>

<div class="history-container">
	<div class="header-container">
		<div class="header-top">
			<h2>History</h2>
			<span class="tx-count">{transfers.length} transactions</span>
		</div>
		<SearchInput />
	</div>
	{#if isLoading}
		<div class="spinner-container">
			<div class="spinner"></div>
		</div>
	{:else if isMobile}
		<MobileHistory {transfersToDisplay} />
	{:else}
		<DefaultHistory {transfersToDisplay} />
	{/if}
	{#if totalPages > 1}
		<div class="pagination-container">
			<button
				on:click={() => {
					currentPage = Math.max(currentPage - 1, 1);
				}}
				disabled={currentPage <= 1}
			>
				&lsaquo;
			</button>
			<span class="page-info">{currentPage} / {totalPages}</span>
			<button
				disabled={currentPage >= totalPages}
				on:click={() => {
					currentPage = Math.min(currentPage + 1, totalPages);
				}}
			>
				&rsaquo;
			</button>
		</div>
	{/if}
</div>

<style>
	.spinner-container {
		display: flex;
		justify-content: center;
		align-items: center;
		flex-grow: 1;
	}

	p {
		word-break: break-all;
		font-size: var(--small-font-size);
		font-family: 'IBM Plex Mono', monospace;
		font-weight: 300;
	}

	.header-container {
		background: transparent;
		padding: 0 0 1em;
		margin-bottom: 0;
		display: flex;
		flex-direction: column;
		gap: 0.6em;
	}

	.header-top {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
	}

	h2 {
		font-size: 1rem;
		font-weight: 700;
		color: var(--c-text);
	}

	.tx-count {
		font-size: 0.6rem;
		font-family: 'IBM Plex Mono', monospace;
		font-weight: 400;
		color: var(--c-grey);
	}

	.spinner {
		width: 8px;
		height: 8px;
		border-color: var(--c-text);
		border-top-color: transparent;
	}

	.history-container {
		display: flex;
		flex-direction: column;
		padding: 1.75em 2em;
		max-width: 1080px;
		width: 90dvw;
		overflow: auto;
		margin-top: 1.5rem;
		background: color-mix(in srgb, var(--c-text) 3%, var(--c-bg));
		border: var(--s-line) solid color-mix(in srgb, var(--c-text) 6%, var(--c-bg));
		border-radius: 3px;
	}

	.pagination-container {
		margin-top: 1em;
		align-self: end;
		display: flex;
		align-items: center;
		gap: 0.25em;
		font-family: 'IBM Plex Mono', monospace;
	}

	.page-info {
		font-size: 0.6rem;
		font-weight: 400;
		color: var(--c-grey);
		min-width: 3em;
		text-align: center;
	}

	.pagination-container button {
		font-size: 0.8em;
		padding: 0.3em 0.6em;
		background: transparent;
		border: var(--s-line) solid color-mix(in srgb, var(--c-text) 12%, var(--c-bg));
		color: var(--c-text);
		border-radius: 2px;
		transition: background 0.1s;
		line-height: 1;
	}

	.pagination-container button:hover:not(:disabled) {
		background: color-mix(in srgb, var(--c-text) 5%, var(--c-bg));
	}
</style>
