<script lang="ts">
	export let value: string;
	export let style: string | undefined;
</script>

<div class="input-container">
	<textarea
		{style}
		placeholder="Send to address"
		title="bridge-input-destination"
		bind:value
		spellcheck="false"
	></textarea>
</div>

<style>
	textarea {
		resize: none;
		overflow: hidden;
		word-break: break-word;
		flex-wrap: wrap;
		place-content: center;

		background-color: #f5f5f5;
		border: none;
		border-radius: 3px;

		padding: 0 1em;
		box-sizing: border-box;
	}

	textarea:focus-visible {
		outline: none;
		box-shadow: none;
	}

	.input-container {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}
</style>
