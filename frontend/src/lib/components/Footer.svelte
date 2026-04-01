<footer class="footer">
	<div class="footer__desktop">
		<span class="footer__brand">
			<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1280 1280" class="footer__logo">
				<path
					fill="currentColor"
					d="M1011.09,268.91V840A171.11,171.11,0,0,1,840,1011.09H440.23c-94.71,0-171.32-76.62-171.32-171.09V440.23A171.2,171.2,0,0,1,440.23,268.91h142.9V440.23H440.23V840H840V594.17H677.37V455.74c143.13-31.49,161-135.13,162.63-173V268.91Z"
				/>
			</svg>
			<strong>1Sec.to</strong>
		</span>
		<nav class="footer__links">
			<a href="https://x.com/1SecBridge" target="_blank" rel="noopener">Twitter</a>
			<a href="https://discord.gg/ytz4nxSsEe" target="_blank" rel="noopener">Discord</a>
			<a href="https://defillama.com/protocol/1sec" target="_blank" rel="noopener">DefiLlama</a>
			<a href="/explorer">Explorer</a>
			<a href="/metadata" target="_blank" rel="noopener">Metadata</a>
		</nav>
	</div>

	<div class="footer__mobile">
		<div class="footer__mobile-top">
			<span class="footer__brand--mobile">
				<svg
					xmlns="http://www.w3.org/2000/svg"
					viewBox="0 0 1280 1280"
					class="footer__logo--mobile"
				>
					<path
						fill="currentColor"
						d="M1011.09,268.91V840A171.11,171.11,0,0,1,840,1011.09H440.23c-94.71,0-171.32-76.62-171.32-171.09V440.23A171.2,171.2,0,0,1,440.23,268.91h142.9V440.23H440.23V840H840V594.17H677.37V455.74c143.13-31.49,161-135.13,162.63-173V268.91Z"
					/>
				</svg>
				<strong>1Sec.to</strong>
			</span>
		</div>

		<nav class="footer__mobile-links">
			<div class="footer__mobile-col">
				<a href="/1transfer">1Transfer</a>
				<a href="/1address">1Address</a>
				<a href="/explorer">Explorer</a>
			</div>
			<div class="footer__mobile-col footer__mobile-col--right">
				<a href="/metadata" target="_blank" rel="noopener">Metadata</a>
				<a href="https://defillama.com/protocol/1sec" target="_blank" rel="noopener">DefiLlama</a>
			</div>
		</nav>

		<div class="footer__mobile-socials">
			<a href="https://x.com/1SecBridge" target="_blank" rel="noopener" aria-label="Twitter">
				<svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor">
					<path
						d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z"
					/>
				</svg>
			</a>
			<a href="https://discord.gg/ytz4nxSsEe" target="_blank" rel="noopener" aria-label="Discord">
				<svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor">
					<path
						d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028c.462-.63.874-1.295 1.226-1.994a.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.892.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.095 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.095 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z"
					/>
				</svg>
			</a>
		</div>

		<p class="footer__mobile-copy">2026 1Sec.to</p>
	</div>
</footer>

<style>
	/* Desktop footer — fixed bar */
	.footer__desktop {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0 calc(var(--s-inset, 1.5rem) * 2);
		height: var(--s-inset, 1.5rem);
		border-top: var(--s-line) solid var(--c-line);
		background: var(--c-bg);
		position: fixed;
		bottom: 0;
		left: 0;
		right: 0;
		z-index: 1;
	}

	.footer__brand {
		display: flex;
		align-items: center;
		gap: 0.35em;
		font-size: 0.7rem;
		color: var(--c-grey);
		font-weight: 100;
	}

	.footer__brand strong {
		font-weight: 600;
	}

	.footer__logo {
		display: inline-flex;
		width: 0.85rem;
		height: 0.85rem;
		color: var(--c-grey);
	}

	.footer__links {
		display: flex;
		gap: 1.5em;
	}

	.footer__links a {
		font-size: 0.65rem;
		color: var(--c-grey);
		text-decoration: none;
		transition: color 0.15s;
	}

	.footer__links a:hover {
		color: var(--c-text);
		text-decoration: none;
	}

	/* Mobile footer — inline section */
	.footer__mobile {
		display: none;
	}

	@media (max-width: 530px) {
		.footer__desktop {
			display: none;
		}

		.footer__mobile {
			display: flex;
			flex-direction: column;
			gap: 1.5em;
			margin-top: 60vh;
			padding: 2em 1.5rem 2.5em;
			border-top: var(--s-line) solid color-mix(in srgb, var(--c-text) 8%, var(--c-bg));
		}

		.footer__mobile-top {
			display: flex;
			align-items: center;
		}

		.footer__brand--mobile {
			display: flex;
			align-items: center;
			gap: 0.4em;
			font-size: 0.85rem;
			color: var(--c-text);
			font-weight: 100;
		}

		.footer__brand--mobile strong {
			font-weight: 700;
		}

		.footer__logo--mobile {
			width: 1.1rem;
			height: 1.1rem;
			color: var(--c-text);
		}

		.footer__mobile-links {
			display: flex;
			justify-content: space-between;
		}

		.footer__mobile-col {
			display: flex;
			flex-direction: column;
			gap: 0.8em;
		}

		.footer__mobile-col--right {
			align-items: flex-end;
		}

		.footer__mobile-col a {
			font-size: 0.8rem;
			font-weight: 400;
			color: var(--c-grey);
			text-decoration: none;
			transition: color 0.15s;
		}

		.footer__mobile-col a:hover {
			color: var(--c-text);
			text-decoration: none;
		}

		.footer__mobile-socials {
			display: flex;
			gap: 1em;
			align-items: center;
		}

		.footer__mobile-socials a {
			color: var(--c-text);
			display: flex;
			align-items: center;
			text-decoration: none;
			transition: opacity 0.15s;
		}

		.footer__mobile-socials a:hover {
			opacity: 0.6;
			text-decoration: none;
		}

		.footer__mobile-copy {
			font-size: 0.65rem;
			color: var(--c-grey);
			font-weight: 400;
		}
	}
</style>
