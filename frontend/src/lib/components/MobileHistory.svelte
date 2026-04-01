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
	import { getTxExplorerUrl, tokenLogoPath, getAccountExplorerUrl } from '$lib/oneSec/config';

	export let transfersToDisplay: TransferInfo[] = [];

	let fromIdToCopy: { [k: number]: boolean } = {};
	let toIdToCopy: { [k: number]: boolean } = {};
</script>

<ul>
	{#each transfersToDisplay as info, id}
		<li>
			{#if info.tx[0]}
				<div class="status">
					<span>Hash:</span>
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
			<div class="status">
				<span>From:</span>
				<a
					target="_blank"
					href={getAccountExplorerUrl(info.sourceChain, info.sourceToken, info.fromAddress)}
				>
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
			</div>
			<div class="status">
				<span>To:</span>
				<a
					target="_blank"
					href={getAccountExplorerUrl(info.destinationChain, info.destinationToken, info.toAddress)}
				>
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
			</div>
			<div class="status">
				<span>Age:</span>
				{#if 'Succeeded' in info.status || 'Failed' in info.status}
					{displayAgeTimestampSeconds(Number(info.ts_ms))}
				{:else}
					processing
				{/if}
			</div>
			<div class="status">
				<span>From token:</span>
				<img
					src={`${tokenLogoPath(info.sourceToken)}`}
					style="width: var(--small-icon-size); height: var(--small-icon-size);"
					alt={info.sourceToken}
				/>
				<div class="status">
					{displayValue(info.deposited)}
					{info.sourceToken}
				</div>
			</div>
			<div class="status">
				<span>To token:</span>
				<img
					src={`${tokenLogoPath(info.destinationToken)}`}
					style="width: var(--small-icon-size); height: var(--small-icon-size);"
					alt={info.destinationToken}
				/>
				<div class="status">
					{displayValue(info.received)}
					{info.destinationToken}
				</div>
			</div>
		</li>
	{/each}
</ul>

<style>
	ul {
		display: flex;
		flex-direction: column;
		margin: 0;
		padding: 0;
		box-sizing: border-box;
	}

	li {
		list-style: none;
		padding: 0.5em;
		border-bottom: 1px solid #e8e5e5;
	}

	.status {
		display: flex;
		align-items: center;
		justify-content: start;
		gap: 0.5em;
		font-size: var(--small-font-size);
		font-family: var(--main-font);
		font-weight: lighter;
	}

	.spinner {
		width: 8px;
		height: 8px;
		border-color: black;
		border-top-color: transparent;
	}

	a {
		align-items: center;
		color: var(--title-color);
		font-family: var(--secondary-font);
		text-decoration: underline;
	}

	span {
		font-size: var(--small-font-size);
	}
</style>
