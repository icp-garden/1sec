<script lang="ts">
	import { displayValue } from '$lib/utils';
	import { Asset, type Chain, type Token, chains } from '$lib/oneSec/types';
	import { chainLogoPath, tokenLogoPath } from '$lib/oneSec/config';
	import { slide } from 'svelte/transition';
	import { createEventDispatcher, onMount } from 'svelte';
	import { user, prices } from '$lib/stores';

	export let isMenu: boolean;
	export let available: Asset[];

	const dispatch = createEventDispatcher();

	// 1. Reactive Price variable
	$: tokenPrices = ($prices?.size ?? 0) === 0 ? undefined : $prices;

	let balances: Map<Token, Map<Chain, number | undefined>> = new Map();
	let totalBalances: Map<Token, number | undefined> = new Map();
	let expandedTokens: { [k: string]: boolean } = {};
	let oneToken = false;
	let listEl: HTMLUListElement;
	let isScrollable = false;
	let scrollPos: 'top' | 'middle' | 'bottom' = 'top';

	function checkScrollable() {
		if (listEl) {
			isScrollable = listEl.scrollHeight > listEl.clientHeight;
		}
	}

	function onScroll() {
		if (!listEl) return;
		const threshold = 4;
		if (listEl.scrollTop <= threshold) {
			scrollPos = 'top';
		} else if (listEl.scrollTop + listEl.clientHeight >= listEl.scrollHeight - threshold) {
			scrollPos = 'bottom';
		} else {
			scrollPos = 'middle';
		}
	}

	// 2. Initialize "expanded" state once on mount
	onMount(() => {
		let uniqueTokens = new Set(available.map((a) => a.token));
		if (uniqueTokens.size === 1) {
			oneToken = true;
			expandedTokens[[...uniqueTokens][0]] = true;
		}
		checkScrollable();
	});

	function toggleToken(token: string) {
		// Filter the available assets to find those matching the clicked token
		const tokenAssets = available.filter((a) => a.token === token);

		// If only 1 chain is available for this token, select it immediately
		// instead of expanding the menu.
		if (tokenAssets.length === 1) {
			dispatch('select', tokenAssets[0]);
			return;
		}

		// Otherwise, toggle the visual expansion
		expandedTokens[token] = !expandedTokens[token];
	}

	// 3. REACTIVE BLOCK: This replaces fetchBalances, setBalances, and the setInterval
	// Whenever $user or available changes, this code runs automatically.
	$: {
		// A. Prepare fresh maps
		let newBalancesMap: Map<Token, Map<Chain, number | undefined>> = new Map();
		let newTotalBalancesMap: Map<Token, number | undefined> = new Map();

		// Initialize structures based on 'available' assets
		available.forEach((asset) => {
			if (!newBalancesMap.has(asset.token)) {
				newBalancesMap.set(asset.token, new Map());
			}
			newBalancesMap.get(asset.token)!.set(asset.chain, undefined);
			newTotalBalancesMap.set(asset.token, undefined);
		});

		// B. Calculate Balances from $user store
		chains.forEach((chain) => {
			// Only check chains the user has enabled
			if (($user.icp && chain === 'ICP') || ($user.evm && chain !== 'ICP')) {
				$user.tokens(chain).forEach((token) => {
					const balanceObj = $user.getBalance(chain, token);

					// Update Detailed Balances
					if (newBalancesMap.has(token)) {
						const chainMap = newBalancesMap.get(token)!;
						// If we have a balance object, use amount, otherwise undefined
						chainMap.set(chain, balanceObj ? balanceObj.amount : undefined);
					}
				});
			}
		});

		// C. Compute Totals
		newBalancesMap.forEach((chainMap, token) => {
			let sum = 0;
			let hasAnyValue = false;

			chainMap.forEach((val) => {
				if (val !== undefined) {
					sum += val;
					hasAnyValue = true;
				}
			});

			// If we found at least one valid number, set the total.
			// Otherwise leave as undefined so UI shows -/- (or handling 0 if preferred)
			newTotalBalancesMap.set(token, hasAnyValue ? sum : undefined);
		});

		// D. Update State
		balances = newBalancesMap;
		totalBalances = newTotalBalancesMap;
	}
</script>

<div class="user-info-container">
	<ul
		bind:this={listEl}
		on:scroll={onScroll}
		style:gap={isMenu ? '0.15em' : '0'}
		class:menu-list={isMenu}
		class:scroll-top={isScrollable && scrollPos === 'top'}
		class:scroll-middle={isScrollable && scrollPos === 'middle'}
		class:scroll-bottom={isScrollable && scrollPos === 'bottom'}
	>
		{#each Array.from(totalBalances.keys()) as token, i}
			{#if available.some((asset) => asset.token === token)}
				<li style={`--i:${i}`} style:padding={isMenu ? '0.1em' : '0'}>
					{#if !oneToken}
						<button
							class="button-white"
							on:click={() => toggleToken(token)}
							style:opacity={Object.values(expandedTokens).some((value) => value) ? '0.5' : '1'}
						>
							<div class="left-container">
								<img class="main-token-logo" src={tokenLogoPath(token)} alt="Logo" />
								<div class="column-layout-container">
									<p class="token-name">{token}</p>
									{#if !isMenu}
										<div class="chain-dots">
											{#each chains.filter( (chain) => available.some((asset) => asset.chain === chain && asset.token === token) ) as chain}
												<img
													class="chain-dot"
													src={chainLogoPath(chain)}
													alt={chain}
													title={chain}
												/>
											{/each}
										</div>
									{/if}
								</div>
							</div>

							<div class="right-container">
								{#if totalBalances.get(token) !== undefined && (totalBalances.get(token) ?? 0) > 0}
									<p>
										{displayValue(totalBalances.get(token) ?? 0)}
									</p>
									{#if tokenPrices && tokenPrices.get(token)}
										<p style="color: grey;">
											${displayValue(
												(totalBalances.get(token) ?? 0) * (tokenPrices.get(token) ?? 0)
											)}
										</p>
									{/if}
								{:else if isMenu}
									<p class="zero-balance">0</p>
								{/if}
							</div>
						</button>
					{/if}

					{#if expandedTokens[token]}
						{#each chains as chain, k (chain + token)}
							{#if available.some((asset) => asset.chain === chain && asset.token === token)}
								<button
									on:click={() => {
										const value = new Asset(chain, token);
										dispatch('select', value);
										toggleToken(token);
									}}
									in:slide={{ duration: 200 }}
									out:slide={{ duration: 200 }}
									style={`--i:${k}`}
									class="chain-row {isMenu ? 'button-hover-in-menu' : ''}"
								>
									<div class="left-container">
										<img class="token-logo" src={chainLogoPath(chain)} alt="Logo" />
										<span class="chain-name">{chain}</span>
									</div>
									<div class="right-container">
										{#if (balances.get(token)?.get(chain) ?? 0) > 0}
											<p>
												{displayValue(balances.get(token)?.get(chain) ?? 0)}
											</p>
											{#if isMenu && tokenPrices && tokenPrices.get(token)}
												<p style="color: grey;">
													${displayValue(
														(balances.get(token)?.get(chain) ?? 0) * (tokenPrices.get(token) ?? 0),
														2
													)}
												</p>
											{/if}
										{/if}
									</div>
								</button>
							{/if}
						{/each}
					{/if}
				</li>
			{/if}
		{/each}
	</ul>
</div>

<style>
	span,
	p {
		text-align: start;
		font-size: 0.7rem;
		margin: 0;
	}

	span {
		color: var(--c-grey);
		font-size: 0.6rem;
	}

	li {
		font-size: var(--normal-font-size);
		padding: 0;
		gap: 0;
		display: flex;
		flex-direction: column;
		border: none;
		border-radius: 2px;
	}

	ul {
		list-style: none;
		margin: 0;
		padding: 0.3rem;
		margin-top: 0.4em;
		width: 100%;
		gap: 0;
		display: flex;
		flex-direction: column;
		background: var(--c-bg);
		border: var(--s-line) solid color-mix(in srgb, var(--c-text) 8%, var(--c-bg));
		border-radius: 3px;
		box-shadow: 0 4px 24px color-mix(in srgb, var(--c-text) 8%, transparent);
		max-height: 18rem;
		overflow-y: auto;
		scrollbar-width: none;
	}

	ul.scroll-top {
		mask-image: linear-gradient(to bottom, black calc(100% - 4rem), transparent);
		-webkit-mask-image: linear-gradient(to bottom, black calc(100% - 4rem), transparent);
	}

	ul.scroll-middle {
		mask-image: linear-gradient(
			to bottom,
			transparent,
			black 4rem,
			black calc(100% - 4rem),
			transparent
		);
		-webkit-mask-image: linear-gradient(
			to bottom,
			transparent,
			black 4rem,
			black calc(100% - 4rem),
			transparent
		);
	}

	ul.scroll-bottom {
		mask-image: linear-gradient(to bottom, transparent, black 4rem);
		-webkit-mask-image: linear-gradient(to bottom, transparent, black 4rem);
	}

	ul.menu-list {
		border: none;
		box-shadow: none;
		padding: 0;
		margin-top: 0;
		max-height: none;
		gap: 0 !important;
	}

	ul.menu-list li {
		border-bottom: var(--s-line) solid color-mix(in srgb, var(--c-text) 5%, var(--c-bg));
	}

	ul.menu-list li:last-child {
		border-bottom: none;
	}

	ul.menu-list button {
		padding: 0.5em 0.25em;
	}

	@keyframes slide-in-small {
		0% {
			opacity: 0;
			transform: translateX(-1em);
		}
		100% {
			opacity: 1;
			transform: translateX(0);
		}
	}

	button {
		display: flex;
		opacity: 0;
		align-items: center;
		width: 100%;
		transform: translateX(-0.5em);
		animation: slide-in-small 300ms var(--tf-snappy);
		animation-fill-mode: forwards;
		animation-delay: calc(var(--inital-delay) + var(--i) * 20ms);
		background: transparent !important;
		border: none !important;
		border-radius: 2px;
		color: var(--c-text) !important;
		padding: 0.55em 0.5em;
		font-size: 0.8rem;
		font-weight: 550;
		transition: background 0.1s;
	}

	button:hover {
		background: color-mix(in srgb, var(--c-text) 5%, var(--c-bg)) !important;
		opacity: 1 !important;
	}

	.button-hover-in-menu:hover {
		opacity: 1 !important;
	}

	.left-container {
		display: flex;
		align-items: center;
		flex: 1;
		gap: 0.5em;
	}

	.zero-balance {
		color: color-mix(in srgb, var(--c-text) 15%, var(--c-bg));
	}

	.right-container {
		display: flex;
		flex-direction: column;
		align-items: end;
		font-size: 0.7em;
		color: var(--c-text);
		font-weight: 400;
		font-family: 'IBM Plex Mono', monospace;
		white-space: nowrap;
	}

	.column-layout-container {
		display: flex;
		flex-direction: column;
		gap: 0.1em;
	}

	.token-name {
		font-weight: 600;
		font-size: 0.8rem;
		color: var(--c-text);
	}

	.chain-dots {
		display: flex;
		gap: 0.25em;
		align-items: center;
	}

	.chain-dot {
		width: 0.75em;
		height: 0.75em;
		border-radius: 50%;
	}

	.chain-row {
		padding-left: 2.8em;
	}

	.chain-name {
		font-size: 0.75rem !important;
		color: var(--c-text) !important;
		font-weight: 450;
	}

	.user-info-container {
		display: flex;
		word-break: break-all;
		flex-direction: column;
		gap: 1em;
		width: 100%;
		padding-bottom: 2em;

		animation: slide-in 300ms var(--tf-snappy);
		animation-fill-mode: forwards;
		animation-delay: 100ms;
		opacity: 0;
		transform: translateX(-5rem);
	}

	@keyframes slide-in {
		0% {
			opacity: 0;
			transform: translateX(-5rem);
		}
		20% {
			opacity: 0;
		}
		100% {
			opacity: 1;
			transform: translateX(0);
		}
	}

	.chain-container {
		position: relative;
	}

	.chain-container .chain-logo {
		position: absolute;
		right: -0.15rem;
		bottom: -0.05rem;
		background-color: var(--c-bg);
		border: 1.5px solid var(--c-bg);
		border-radius: 50%;
		padding: 0;
	}

	.main-token-logo {
		width: 2em;
		height: 2em;
		border-radius: 50%;
	}

	.token-logo {
		width: 1.4em;
		height: 1.4em;
		border-radius: 50%;
	}

	.chain-logo {
		width: 0.7em;
		height: 0.7em;
	}

	@keyframes slide-in-chain {
		0% {
			opacity: 0;
			transform: translateX(-0.5em);
		}
		100% {
			opacity: 1;
			transform: translateX(calc(var(--j) * -0.3em));
		}
	}

	.chain-logo-main {
		z-index: calc(100 - var(--j));
		opacity: 0;
		transform: translateX(-0.5em);
		background-color: var(--c-bg);
		border: 1px solid var(--c-bg);
		border-radius: 50%;
		width: 0.9em;
		height: 0.9em;
		animation: slide-in-chain 200ms var(--tf-snappy);
		animation-fill-mode: forwards;
		animation-delay: calc(100ms + var(--j) * 20ms);
	}
</style>
