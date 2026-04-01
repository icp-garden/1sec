import { fileURLToPath, URL } from 'url';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import inject from '@rollup/plugin-inject';
import dotenv from 'dotenv';

dotenv.config();

export default defineConfig({
	build: {
		emptyOutDir: true,
		rollupOptions: {
			plugins: [
				inject({
					modules: { Buffer: ['buffer', 'Buffer'] }
				})
			]
		}
	},
	optimizeDeps: {
		esbuildOptions: {
			define: {
				global: 'globalThis'
			}
		}
	},
	server: {
		port: 3847,
		proxy: {
			'/api': {
				target: 'http://127.0.0.1:9123',
				changeOrigin: true
			}
		}
	},
	// Polyfill Buffer for production build.
	plugins: [sveltekit()],
	resolve: {
		alias: [
			{
				find: '$lib',
				replacement: fileURLToPath(new URL('./src/lib', import.meta.url))
			}
		],
		extensions: ['.js', '.json', '.ts', '.svelte', '.did.d.ts']
	}
});
