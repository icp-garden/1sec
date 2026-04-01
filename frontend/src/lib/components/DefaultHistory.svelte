<script lang="ts">
	import CheckIcon from '$lib/icons/CheckIcon.svelte';
	import CopyIcon from '$lib/icons/CopyIcon.svelte';
	import SuccessIcon from '$lib/icons/SuccessIcon.svelte';
	import WarningIcon from '$lib/icons/WarningIcon.svelte';
	import {
		displayAgeTimestampSeconds,
		displayValue,
		displayTx,
		displayTxWithInfo,
		truncateAddress
	} from '$lib/utils';
	import type { TransferInfo } from '$lib/oneSec/types';
	import {
		getAccountExplorerUrl,
		getTxExplorerUrl,
		tokenLogoPath,
		chainLogoPath
	} from '$lib/oneSec/config';

	export let transfersToDisplay: TransferInfo[] = [];

	let fromIdToCopy: { [k: number]: boolean } = {};
	let toIdToCopy: { [k: number]: boolean } = {};
</script>

<div class="table-content-container">
	<table>
		<thead>
			<tr>
				<th>Tx</th>
				<th>From</th>
				<th>To</th>
				<th>Age</th>
				<th>Sent</th>
				<th>Received</th>
			</tr>
		</thead>
		<tbody>
			{#each transfersToDisplay as info, id}
				<tr>
					<td>
						{#if info.tx[0]}
							<div class="status">
								<div class="status">
									{#if 'Succeeded' in info.status}
										<SuccessIcon size="small" />
									{:else if 'Failed' in info.status}
										<WarningIcon size="small" />
									{:else}
										<div class="spinner"></div>
									{/if}
								</div>
								<a
									target="_blank"
									href={getTxExplorerUrl(
										info.destinationChain,
										info.destinationToken,
										displayTx(info.tx[0])
									)}
								>
									{displayTx(info.tx[0]).length > 32
										? displayTx(info.tx[0]).slice(0, 25) + '...'
										: displayTx(info.tx[0])}
								</a>
							</div>
						{/if}
					</td>
					<td>
						<div class="status">
							<a href={`?address=${info.fromAddress}`}>
								{truncateAddress(info.fromAddress)}
							</a>
							<button
								class="copy-btn"
								on:click={() => {
									fromIdToCopy[id] = true;
									navigator.clipboard.writeText(info.fromAddress);
									setTimeout(() => (fromIdToCopy[id] = false), 800);
								}}
							>
								{#if fromIdToCopy[id]}
									<CheckIcon color="black" size="small" />
								{:else}
									<CopyIcon color="black" size="small" />
								{/if}
							</button>
							<a
								target="_blank"
								href={getAccountExplorerUrl(info.sourceChain, info.sourceToken, info.fromAddress)}
							>
							</a>
						</div>
					</td>
					<td>
						<div class="status">
							<a href={`?address=${info.toAddress}`}>
								{truncateAddress(info.toAddress)}
							</a>
							<button
								class="copy-btn"
								on:click={() => {
									toIdToCopy[id] = true;
									navigator.clipboard.writeText(info.toAddress);
									setTimeout(() => (toIdToCopy[id] = false), 800);
								}}
							>
								{#if toIdToCopy[id]}
									<CheckIcon color="black" size="small" />
								{:else}
									<CopyIcon color="black" size="small" />
								{/if}
							</button>
							<a
								target="_blank"
								href={getAccountExplorerUrl(
									info.destinationChain,
									info.destinationToken,
									info.toAddress
								)}
							>
							</a>
						</div>
					</td>
					<td>
						<div class="status">
							{#if 'Succeeded' in info.status || 'Failed' in info.status}
								{displayAgeTimestampSeconds(Number(info.ts_ms))}
							{:else}
								processing
							{/if}
						</div>
					</td>
					<td>
						<div style="display: flex; align-items:center;">
							<div>
								<img
									src={`${tokenLogoPath(info.sourceToken)}`}
									width="25px"
									height="25px"
									alt={info.sourceToken}
								/>
								<img
									src={`${chainLogoPath(info.sourceChain)}`}
									width="15px"
									height="15px"
									class="chain-img"
									alt={info.sourceChain}
								/>
							</div>
							<div class="status">
								{displayValue(info.deposited)}
								{info.sourceToken}
							</div>
						</div>
					</td>
					<td>
						<div style="display: flex; align-items:center;">
							<div>
								<img
									src={`${tokenLogoPath(info.destinationToken)}`}
									width="25px"
									height="25px"
									alt={info.destinationToken}
								/>
								<img
									src={`${chainLogoPath(info.destinationChain)}`}
									width="15px"
									height="15px"
									class="chain-img"
									alt={info.destinationChain}
								/>
							</div>
							<div class="status">
								{displayValue(info.received)}
								{info.destinationToken}
							</div>
						</div>
					</td>
				</tr>
			{/each}
		</tbody>
	</table>
	{#if transfersToDisplay.length == 0}
		<div style="display: flex; place-content: center; height: 250px; align-items: center;">
			<h2>There are no matching entries</h2>
		</div>
	{/if}
</div>

<style>
	table {
		font-family: inherit;
		border-collapse: separate;
		border-spacing: 0;
		font-size: 0.75rem;
		width: 100%;
	}

	.table-content-container {
		margin-top: 0.5em;
		min-width: max-content;
		overflow: auto;
	}

	th {
		text-align: left;
		padding: 1em 0.75em;
		font-size: 0.65rem;
		font-weight: 600;
		color: var(--c-text);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		border-bottom: 2px solid color-mix(in srgb, var(--c-text) 10%, var(--c-bg));
	}

	td {
		text-align: left;
		padding: 1.1em 0.75em;
		border-bottom: var(--s-line) solid color-mix(in srgb, var(--c-text) 5%, var(--c-bg));
		font-size: 0.7rem;
		vertical-align: middle;
	}

	tr:hover td {
		background: color-mix(in srgb, var(--c-text) 2%, var(--c-bg));
	}

	button {
		color: var(--c-text) !important;
		border: none !important;
		background: transparent !important;
		padding: 0;
		cursor: pointer;
	}

	button:disabled {
		opacity: 0.4;
	}

	.chain-img {
		position: relative;
		top: 5px;
		left: -10px;
		background: var(--c-bg);
		padding: 1px;
		border-radius: 50%;
	}

	.status {
		display: flex;
		align-items: center;
		justify-content: start;
		gap: 0.35em;
		font-family: 'IBM Plex Mono', monospace;
		font-size: 0.65rem;
		font-weight: 400;
		white-space: nowrap;
	}

	.spinner {
		width: 8px;
		height: 8px;
		border-color: var(--c-text);
		border-top-color: transparent;
	}

	a {
		display: inline-flex;
		align-items: center;
		gap: 0.2em;
		color: var(--c-text);
		font-family: 'IBM Plex Mono', monospace;
		font-size: 0.65rem;
		text-decoration: none;
		white-space: nowrap;
	}

	a:hover {
		text-decoration: underline;
		color: var(--c-text--interactive);
	}

	h2 {
		font-size: 0.85rem;
		font-weight: 400;
		color: var(--c-grey);
	}
</style>
