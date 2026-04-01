<script lang="ts">
	import Deposit from '$lib/components/Deposit.svelte';
</script>

<div class="main-container">
	<h2 class="headline">Deposit from any chain.</h2>
	<Deposit />
</div>

<style>
	.headline {
		font-weight: 700;
		font-size: 1.25rem;
		margin: 0;
	}

	.main-container {
		display: flex;
		align-items: center;
		flex-direction: column;
		width: 100%;
		max-width: 26rem;
		padding-top: 2vh;
	}
</style>
