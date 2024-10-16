import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

const port = process.env.RADROOTS_APP_PORT ? Number(process.env.RADROOTS_APP_PORT) : 3000;
const host = process.env.TAURI_DEV_HOST ? process.env.TAURI_DEV_HOST : false;

export default defineConfig({
	plugins: [
		sveltekit()
	],
	clearScreen: false,
	server: {
		port,
		strictPort: true,
		host,
		hmr: host
			? {
				protocol: "ws",
				host,
				port: port + 1,
			}
			: undefined,
		watch: {
			ignored: [
				"**/target/**",
				"**/crates/**",
			],
		},
	},
});
