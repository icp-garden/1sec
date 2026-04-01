<script lang="ts">
	import { icpAnonymous } from '$lib/user/icpUser';
	import * as fromCandid from '$lib/oneSec/fromCandid';
	import type { EvmChainMetadata, TokenMetadata } from '../../declarations/one_sec/one_sec.did';
	import { CONFIG } from '$lib/oneSec/config';

	const fetchMetadata = async () => {
		const result = await icpAnonymous().oneSec().get_metadata();
		switch (true) {
			case 'Ok' in result: {
				for (let tc of result.Ok.tokens) {
					const candidChain = tc.chain[0];
					const candidToken = tc.token[0];
					if (
						candidChain === undefined ||
						candidToken === undefined ||
						fromCandid.token(candidToken) === 'ckBTC'
					) {
						continue;
					}
					const chain = fromCandid.chain(candidChain);
					const token = fromCandid.token(candidToken);
					if (chain === 'ICP') {
						const config = CONFIG.icp.token.get(token);
						if (!config) {
							console.error(`No config for ${chain} ${token}`);
							continue;
						}
						if (config.ledger.toText() != tc.contract) {
							console.error(
								`Mismatch in ledger address: ${chain} ${token}: ${config.ledger.toText()} vs ${tc.contract}`
							);
						}
					} else {
						const config = CONFIG.evm.get(chain)!.token.get(token);
						if (!config) {
							console.error(`No config for ${chain} ${token}`);
							continue;
						}
						if (config.erc20 != tc.contract) {
							console.error(
								`Mismatch in contract address: ${chain} ${token}: ${config.erc20} vs ${tc.contract}`
							);
						}
						if (config.locker != tc.locker[0]) {
							console.error(
								`Mismatch in contract locker: ${chain} ${token}: ${config.locker} vs ${tc.locker[0]}`
							);
						}
					}
				}
				return result.Ok;
			}
			case 'Err' in result:
				throw Error(result.Err);
			default: {
				const _unreachable: never = result;
				throw Error('unreachable');
			}
		}
	};

	function sortTokens(tokens: TokenMetadata[]): TokenMetadata[] {
		tokens = tokens.filter((t) => t.token[0] !== undefined && t.chain[0] !== undefined);
		tokens.sort((a, b) => {
			const ta = fromCandid.token(a.token[0]!);
			const tb = fromCandid.token(b.token[0]!);
			const r = ta.localeCompare(tb);
			if (r != 0) {
				return r;
			} else {
				const ca = fromCandid.chain(a.chain[0]!);
				const cb = fromCandid.chain(b.chain[0]!);
				return ca.localeCompare(cb);
			}
		});
		return tokens;
	}

	function sortChains(chains: EvmChainMetadata[]): EvmChainMetadata[] {
		chains = chains.filter((c) => c.chain[0] !== undefined);
		chains.sort((a, b) => {
			const ca = fromCandid.chain(a.chain[0]!);
			const cb = fromCandid.chain(b.chain[0]!);
			return ca.localeCompare(cb);
		});
		return chains;
	}
</script>

<div class="container">
	{#await fetchMetadata()}
		Loading..
	{:then m}
		<h2>Metadata</h2>
		<table>
			<tbody>
				<tr>
					<td>Last upgrade</td>
					<td>{new Date(Number(m.last_upgrade_time)).toLocaleString()}</td>
				</tr>
				<tr>
					<td>Cycles balance</td>
					<td>{Math.round((Number(m.cycle_balance) / Math.pow(10, 12)) * 100) / 100} TC</td>
				</tr>
				<tr>
					<td>Stable memory</td>
					<td>{Number(m.stable_memory_bytes) / 1024} KiB</td>
				</tr>
				<tr>
					<td>Wasm memory</td>
					<td>{Number(m.wasm_memory_bytes) / 1024} KiB</td>
				</tr>
				<tr>
					<td>Number of events</td>
					<td>{m.event_count}</td>
				</tr>
				<tr>
					<td>Size of events</td>
					<td>{Math.round(Number(m.event_bytes) / 1024)} KiB</td>
				</tr>
			</tbody>
		</table>

		{#each sortTokens(m.tokens).filter( (m) => (m.token.length === 1 ? fromCandid.token(m.token[0]) !== 'ckBTC' : false) ) as token}
			<h2>{fromCandid.token(token.token[0]!)} on {fromCandid.chain(token.chain[0]!)}</h2>
			<table>
				<tbody>
					<tr>
						<td> Contract </td>
						<td>
							{token.contract}
						</td>
					</tr>
					{#if token.locker[0]}
						<tr>
							<td> Helper </td>
							<td>
								{token.locker}
							</td>
						</tr>
					{/if}
					<tr>
						<td> Balance </td>
						<td>
							{Math.round((Number(token.balance) / Math.pow(10, token.decimals)) * 1000) / 1000}
						</td>
					</tr>
					<tr>
						<td> Queue size </td>
						<td>
							{token.queue_size}
						</td>
					</tr>
				</tbody>
			</table>
		{/each}

		{#each sortChains(m.evm_chains) as evm}
			<h2>{fromCandid.chain(evm.chain[0]!)}</h2>
			<table>
				<tbody>
					<tr>
						<td> Chain id </td>
						<td>
							{evm.chain_id}
						</td>
					</tr>
					<tr>
						<td> Nonce </td>
						<td>
							{evm.nonce}
						</td>
					</tr>
					<tr>
						<td> Block time, ms </td>
						<td>
							{evm.block_time_ms}
						</td>
					</tr>
					<tr>
						<td> Block number, safe </td>
						<td>
							{evm.block_number_safe}
						</td>
					</tr>
					<tr>
						<td> Block number, latest </td>
						<td>
							{evm.block_number_latest}
						</td>
					</tr>
					{#if Number(evm.fetch_time_safe_ms)}
						<tr>
							<td>Fetch time, safe</td>
							<td>{new Date(Number(evm.fetch_time_safe_ms)).toLocaleString()}</td>
						</tr>
					{/if}
					{#if Number(evm.fetch_time_latest_ms)}
						<tr>
							<td>Fetch time, latest</td>
							<td>{new Date(Number(evm.fetch_time_latest_ms)).toLocaleString()}</td>
						</tr>
					{/if}
					<tr>
						<td> Max fee per gas </td>
						<td>
							{evm.max_fee_per_gas}
						</td>
					</tr>
					<tr>
						<td> Max priority fee per gas </td>
						<td>
							{evm.max_priority_fee_per_gas}
						</td>
					</tr>
				</tbody>
			</table>
		{/each}
	{:catch error}
		{error}
	{/await}
</div>

<style>
	h2 {
		margin-top: 24px;
		margin-bottom: 16px;
	}
	.container {
		max-width: 1080px;
		width: 90dvw;
		background-color: var(--container-background-color);
		display: flex;
		flex-direction: column;
		padding: 1em;
		border-radius: 3px;
		max-width: 1080px;
		width: 90dvw;
		overflow: auto;
		margin-top: 1rem;
	}
	/* === Base Styles === */
	table {
		font-family: monospace;
		border-collapse: separate;
		border-spacing: 0;
		border: 1px solid black;
		font-size: var(--small-font-size);
		background-color: var(--container-background-color);
		width: 100%;
	}

	tr td:first-child {
		width: 200px;
	}

	td {
		text-align: left;
		background-color: var(--input-color);
		border: 1px solid var(--container-background-color);
		padding: 10px;
	}

	tr:nth-child(even) {
		background-color: var(--container-background-color);
	}

	tr:nth-child(odd) {
		background-color: #eeeeee;
	}
</style>
