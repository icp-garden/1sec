<script lang="ts">
	import '../app.css';
	import { showingModalDialog, user, prices } from '$lib/stores';
	import { onMount } from 'svelte';
	import Navbar from '$lib/components/Navbar.svelte';
	import { tryReconnectEvm, tryReconnectInternetIdentity } from '$lib/wallet/wallet';
	import { BALANCE_UPDATE_MS } from '$lib/types';
	import { updateBalances } from '$lib/utils';
	import { fetchPriceCoinGecko } from '$lib/oneSec/utils';
	import Toast from '$lib/components/Toast.svelte';
	import Footer from '$lib/components/Footer.svelte';

	let timerId: NodeJS.Timeout | undefined = undefined;

	onMount(() => {
		fetchPriceCoinGecko().then((priceMap) => {
			console.log('Prices fetched:', priceMap); // Check your BROWSER console now
			prices.set(priceMap);
		});

		if (!$user.icp) {
			tryReconnectInternetIdentity().then((userLocal) => {
				$user.icp = userLocal;
				updateBalances($user.icp, $user.evm);
			});
		}
		if (!$user.evm) {
			tryReconnectEvm().then((userLocal) => {
				$user.evm = userLocal;
				updateBalances($user.icp, $user.evm);
			});
		}

		timerId = setInterval(async () => {
			await updateBalances($user.icp, $user.evm);
		}, BALANCE_UPDATE_MS);

		return () => clearInterval(timerId);
	});
</script>

<div class="page-container">
	<Navbar />
	<div class="content-container">
		<slot />
	</div>
	{#if !$showingModalDialog}
		<Toast />
	{/if}
	<Footer />
</div>

<style>
	.page-container {
		display: flex;
		flex-direction: column;
		height: fit-content;
		min-height: 100%;
		width: 100vw;
		background: var(--c-bg);
		overflow: auto;
	}

	.content-container {
		margin-top: 1em;
		display: flex;
		flex-direction: column;
		justify-content: flex-start;
		align-items: center;
		z-index: 2;
		padding: 0 1rem;
		flex: 1;
	}

	@view-transition {
		navigation: auto;
	}
</style>
