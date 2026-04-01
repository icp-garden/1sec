<script lang="ts">
	import { page } from '$app/stores';
	import { user, showingModalDialog, showLoginSidebar } from '$lib/stores';
	import PlusIcon from '$lib/icons/PlusIcon.svelte';
	import Address from './Address.svelte';
	import WalletIcon from '../icons/WalletIcon.svelte';
	import Menu from './Menu.svelte';
	import { onMount } from 'svelte';

	let showWalletMenu = false;
	let windowWidth = 0;

	function openMenu() {
		showWalletMenu = true;
		$showingModalDialog = true;
	}

	// React to showLoginSidebar store being set from other components
	$: if ($showLoginSidebar) {
		showWalletMenu = true;
		$showingModalDialog = true;
		$showLoginSidebar = false;
	}

	onMount(() => {
		handleResize();
	});

	window.addEventListener('resize', handleResize);

	function handleResize() {
		windowWidth = window.innerWidth;
	}
</script>

<nav>
	<div class="left-group">
		<a href="/" class="logo-link">
			<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1280 1280" width="1.5em" height="1.5em"
				><g id="ONE_1_SHAPE" data-name="ONE/1 SHAPE"
					><path
						fill="var(--c-text)"
						d="M1011.09,268.91V840A171.11,171.11,0,0,1,840,1011.09H440.23c-94.71,0-171.32-76.62-171.32-171.09V440.23A171.2,171.2,0,0,1,440.23,268.91h142.9V440.23H440.23V840H840V594.17H677.37V455.74c143.13-31.49,161-135.13,162.63-173V268.91Z"
					/></g
				></svg
			>
			<h1>1Sec.to</h1>
		</a>
		<a
			href="/1transfer"
			class="nav-link"
			class:nav-link--active={$page.url.pathname.startsWith('/1transfer')}>1Transfer</a
		>
		<a
			href="/1address"
			class="nav-link"
			class:nav-link--active={$page.url.pathname.startsWith('/1address')}>1Address</a
		>
		<a
			href="/explorer"
			class="nav-link"
			class:nav-link--active={$page.url.pathname.startsWith('/explorer')}>1Explorer</a
		>
	</div>

	<div class="right-container">
		{#if $user.icp || $user.evm}
			{#if $user.icp}
				<Address
					address={$user.icp.principal.toString()}
					short={true}
					bind:showWalletMenu
					triggerWalletMenu={true}
					style="display: {windowWidth > 500 || windowWidth === 0 ? 'flex' : 'none'};"
					size="tiny"
					allowCopy={windowWidth === 0 || windowWidth > 500}
					imgSrc={$user.icp.wallet.icon}
				/>
			{/if}
			{#if $user.evm}
				<Address
					address={$user.evm.address}
					short={true}
					bind:showWalletMenu
					style="display: {windowWidth > 500 || windowWidth === 0 ? 'flex' : 'none'};"
					size="tiny"
					triggerWalletMenu={true}
					allowCopy={windowWidth === 0 || windowWidth > 500}
					imgSrc={$user.evm.wallet.icon}
				/>
			{/if}
			<button class="menu-icon-btn" on:click={openMenu}>
				<WalletIcon size="large" />
			</button>
		{:else}
			<button class="pill-btn" on:click={openMenu}> Login </button>
		{/if}
	</div>
</nav>

{#if showWalletMenu}
	<Menu on:close={() => (showWalletMenu = false)} />
{/if}

<style>
	h1 {
		color: var(--c-text);
		font-size: 0.9rem;
		font-weight: 700;
		letter-spacing: -0.03em;
	}

	nav {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem 1.5rem;
		border-bottom: none;
		background: var(--c-bg);
		gap: 0.75rem;
	}

	div {
		display: flex;
		align-items: center;
	}

	a {
		display: flex;
		align-items: center;
		text-decoration: none;
		color: var(--c-text);
		gap: 0.4em;
	}

	a:hover {
		text-decoration: none;
	}

	button {
		display: flex;
		align-items: center;
		font-size: 0.75em;
		padding: 0.5ex 1.5ch;
	}

	button:hover {
		opacity: 0.8;
	}

	.left-group {
		display: flex;
		align-items: center;
		gap: 1.5em;
		overflow: hidden;
		min-width: 0;
	}

	.logo-link {
		display: flex;
		align-items: center;
		gap: 0.3em;
		text-decoration: none;
		color: var(--c-text);
	}

	.logo-link:hover {
		text-decoration: none;
	}

	.right-container {
		color: var(--c-text);
		gap: 0.5rem;
	}

	.menu-icon-btn {
		background: transparent !important;
		border: none !important;
		color: var(--c-text) !important;
		padding: 0.3em;
		border-radius: 50%;
		cursor: pointer;
	}

	.menu-icon-btn:hover {
		background: color-mix(in srgb, var(--c-text) 8%, transparent) !important;
	}

	.pill-btn {
		background: var(--c-text);
		color: var(--c-bg);
		border: none;
		border-radius: 3px;
		padding: 0.4em 1.2em;
		font-size: 0.75rem;
		font-weight: 600;
		cursor: pointer;
		transition: opacity 0.15s;
	}

	.pill-btn:hover {
		opacity: 0.8;
	}

	.nav-link {
		font-size: 0.8rem;
		font-weight: 100;
		color: var(--c-text);
		text-decoration: none;
	}

	.nav-link:hover {
		text-decoration: none;
	}

	.nav-link--active {
		font-weight: 600;
	}

	@media (max-width: 530px) {
		h1 {
			display: none;
		}

		nav {
			padding: 0.6rem 0.75rem;
		}

		.left-group {
			gap: 0.75em;
		}

		.nav-link {
			display: none;
		}

		.logo-link svg {
			width: 2.2em;
			height: 2.2em;
		}

		.pill-btn {
			font-size: 0.85rem;
			padding: 0.5em 1.4em;
		}

		.menu-icon-btn {
			padding: 0.5em;
		}

		.menu-icon-btn :global(svg) {
			width: var(--huge-icon-size) !important;
			height: var(--huge-icon-size) !important;
		}
	}
</style>
