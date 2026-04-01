<script lang="ts">
	import DownIcon from '$lib/icons/DownIcon.svelte';
	import { chainLogoPath, tokenLogoPath } from '$lib/oneSec/config';
	import { Asset } from '$lib/oneSec/types';

	let { value, pressed, onClick }: { value: Asset; pressed: boolean; onClick: () => void } =
		$props();
</script>

<div class="container">
	<button class="asset-btn {pressed ? 'pressed' : ''}" onclick={onClick}>
		<div class="asset-container">
			<div class="token-logo-container">
				<img class="token-logo" src={tokenLogoPath(value.token)} alt="Logo" />
				<div class="chain-logo-container">
					<img class="chain-logo" src={chainLogoPath(value.chain)} alt="Logo" />
				</div>
			</div>
			<div class="info-container">
				<span>{value.token}</span>
				<span class="info-chain">on {value.chain}</span>
			</div>
		</div>
		<div class="hide-on-small">
			<DownIcon down={!pressed} size="small" color="black" animation="none" />
		</div>
	</button>
</div>

<style>
	.container {
		display: flex;
		flex: 1;
		flex-direction: column;
		justify-content: center;
	}

	.asset-container {
		display: flex;
		align-items: center;
	}

	.info-container {
		display: flex;
		flex-direction: column;
		align-items: start;
		gap: 0.1em;
		padding: 0 0.35em;
	}

	.info-container span {
		font-size: 0.7rem;
		font-weight: 650;
	}

	.info-chain {
		color: var(--c-grey);
		font-size: 0.5rem;
		width: max-content;
		font-weight: 400 !important;
	}

	button {
		display: flex;
		align-items: center;
		width: fit-content;
		gap: 0.2em;
		background: var(--c-bg) !important;
		color: var(--c-text) !important;
		border: var(--s-line) solid color-mix(in srgb, var(--c-text) 10%, var(--c-bg)) !important;
		border-radius: 3px;
		padding: 0.25em 0.45em 0.25em 0.3em;
		cursor: pointer;
		transition:
			background 0.15s,
			box-shadow 0.15s;
		box-shadow: 0 1px 3px color-mix(in srgb, var(--c-text) 5%, transparent);
		line-height: 1;
	}

	@media (max-width: 399px) {
		.hide-on-small {
			display: none;
		}
	}

	button:hover {
		background: color-mix(in srgb, var(--c-text) 4%, var(--c-bg)) !important;
		box-shadow: 0 1px 4px color-mix(in srgb, var(--c-text) 10%, transparent);
	}

	.pressed {
		opacity: 0.6;
		background: color-mix(in srgb, var(--c-text) 4%, var(--c-bg)) !important;
	}

	.chain-logo-container {
		background: var(--c-bg);
		border: 1.5px solid var(--c-bg);
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		position: absolute;
		right: -4px;
		bottom: -4px;
		z-index: 2;
	}

	.chain-logo {
		width: 0.8em;
		height: 0.8em;
	}

	.token-logo-container {
		display: flex;
		align-items: center;
		justify-content: center;
		position: relative;
		padding: 0;
	}

	.token-logo {
		width: 1.6em;
		height: 1.6em;
	}
</style>
