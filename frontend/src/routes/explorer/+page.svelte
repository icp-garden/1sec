<script lang="ts">
	import { page } from '$app/stores';
	import TransactionHistory from '$lib/components/TransactionHistory.svelte';

	$: address = $page.url.searchParams.get('address') || '';
</script>

<TransactionHistory {address} />
