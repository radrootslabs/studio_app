import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from "@tailwindcss/vite";
import { config as dotenv_config } from "dotenv";
import path from "node:path";
import { defineConfig } from "vite";

export default defineConfig(({ mode }) => {
	dotenv_config({ path: mode === "development" ? ".env.development" : ".env.production" });
	return {
		build: {
			sourcemap: true
		},
		plugins: [
			tailwindcss(),
			sveltekit(),
		],
		define: {
			'process.env.NODE_ENV': '"production"',
		},
		server: {
			port: process.env.PORT ? Number(process.env.PORT) : 3000,
			fs: {
				allow: [
					path.resolve(__dirname, ".."),
					path.resolve(__dirname, "../..")

				]
			}
		}
	};
});
