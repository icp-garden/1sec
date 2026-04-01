<script lang="ts">
	import BridgeForm from '$lib/components/BridgeForm.svelte';
</script>

<div class="main-container">
	<h2 class="headline">Bridge anytime, anywhere.</h2>
	<BridgeForm />
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
