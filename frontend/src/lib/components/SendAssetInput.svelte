<script lang="ts">
	let { value = $bindable() }: { value: number | undefined } = $props();
</script>

<div class="input-container">
	<input
		class="input"
		type="number"
		inputmode="decimal"
		autocomplete="off"
		bind:value
		placeholder="0.00"
		min="0"
	/>
</div>

<style>
	input[type='number']::-webkit-inner-spin-button,
	input[type='number']::-webkit-outer-spin-button {
		-webkit-appearance: none;
		margin: 0;
	}

	.input {
		border: none;
		padding: 0;
		font-size: var(--normal-font-size);
		width: 100%;
		background: none;
		outline: none;
		flex-grow: 0;
		text-align: left;
	}

	.input-container {
		margin-left: 10px;
		flex: 1;
	}
</style>
