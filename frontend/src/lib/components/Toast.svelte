<script lang="ts">
	import { toasts } from '$lib/stores';
	import { fade } from 'svelte/transition';
	import { flip } from 'svelte/animate';
	import ErrorIcon from '$lib/icons/ErrorIcon.svelte';
	import CloseIcon from '$lib/icons/CloseIcon.svelte';
	import SuccessIcon from '$lib/icons/SuccessIcon.svelte';
	import WarningIcon from '$lib/icons/WarningIcon.svelte';
	import { onMount } from 'svelte';
	import { TOAST_LIFETIME_MS } from '$lib/toast';

	const REFRESH_RATE_MS = 10;

	let toastContainers: { [key: number]: HTMLDivElement } = {};
	let timeBars: { [key: number]: HTMLDivElement } = {};

	onMount(() => {
		const intervalId = setInterval(() => {
			$toasts.forEach((toast) => {
				if (toast.isTemporary) {
					if (toast.timeLeft > 0) {
						toast.decreaseTime(REFRESH_RATE_MS);
						handleElapsedBarWidth(toast.id, toast.timeLeft);
					} else {
						toasts.remove(toast.id);
					}
				}
			});
		}, REFRESH_RATE_MS);
		return () => clearInterval(intervalId);
	});

	function handleElapsedBarWidth(id: number, timeLeft: number) {
		const toastContainer = toastContainers[id];
		const bar = timeBars[id];
		if (!bar) return;
		bar.style.width =
			(toastContainer.clientWidth * (timeLeft / TOAST_LIFETIME_MS) - 16).toString() + 'px';
	}
</script>

<div class="toasts-container">
	{#each $toasts as toast (toast.id)}
		<div class="toast-container" animate:flip transition:fade>
			<div bind:this={toastContainers[toast.id]} class="toast-content-container">
				{#if toast.type === 'success'}
					<SuccessIcon size="normal" />
				{:else if toast.type === 'error'}
					<ErrorIcon size="normal" />
				{:else}
					<WarningIcon size="normal" />
				{/if}
				<p title="toast-message">{@html toast.message}</p>
				<button
					class="toast-close"
					on:click={() => {
						toasts.remove(toast.id);
					}}
				>
					<CloseIcon color="black" size="normal" />
				</button>
			</div>
			<div
				bind:this={timeBars[toast.id]}
				class="elapsed-bar"
				class:warning={toast.type === 'warning'}
				class:error={toast.type === 'error'}
				class:success={toast.type === 'success'}
			></div>
		</div>
	{/each}
</div>

<style>
	.toasts-container {
		position: absolute;
		bottom: 0;
		left: 0;
		z-index: 10;

		display: flex;
		flex-direction: column-reverse;
		align-items: center;

		width: 100dvw;
		gap: 0.5em;
		margin-bottom: 1em;
	}

	p {
		font-family: 'IBM Plex Mono', monospace;
		font-size: 0.8em;
		font-weight: 300;
		color: var(--c-text);

		flex-grow: 1;
		word-break: keep-all;
	}

	.toast-container {
		display: flex;
		flex-direction: column;
		align-items: start;
	}

	.toast-content-container {
		background: var(--c-bg);
		border: var(--s-line) solid color-mix(in srgb, var(--c-text) 10%, var(--c-bg));
		border-radius: 3px;
		box-shadow: 0 4px 16px color-mix(in srgb, var(--c-text) 10%, transparent);

		color: var(--c-text);

		display: flex;
		align-items: center;

		max-width: 90vw;
		gap: 0.5em;
		box-sizing: border-box;
		padding: 0.7em 1em;

		transition: background 0.15s;
	}

	.toast-content-container:hover {
		background: color-mix(in srgb, var(--c-text) 3%, var(--c-bg));
	}

	.elapsed-bar {
		background: var(--c-grey);
		margin-left: 1em;
		height: 1.5px;
		border-radius: 1px;
	}

	.warning {
		background: var(--c-orange);
	}

	.error {
		background: var(--c-red);
	}

	.success {
		background: var(--c-green);
	}

	.toast-close {
		background: transparent !important;
		border: none !important;
		color: var(--c-text) !important;
		padding: 0.2em;
		cursor: pointer;
		border-radius: 50%;
		display: flex;
		align-items: center;
	}

	.toast-close:hover {
		background: color-mix(in srgb, var(--c-text) 8%, transparent) !important;
	}
</style>
