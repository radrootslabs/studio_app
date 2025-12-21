import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from "@tailwindcss/vite";
import { config as dotenv_config } from "dotenv";
import { execSync } from "node:child_process";
import { readFileSync } from "node:fs";
import path from "node:path";
import { defineConfig } from "vite";

export default defineConfig(({ mode }) => {
	dotenv_config({ path: mode === "development" ? ".env.development" : ".env.production" });
	const repo_root = path.resolve(__dirname, "..");
	const app_package_path = path.resolve(__dirname, "package.json");
	const app_package = JSON.parse(readFileSync(app_package_path, "utf8")) as { name?: string; version?: string };
	const git_hash = (() => {
		try {
			return execSync("git rev-parse HEAD", { cwd: repo_root }).toString().trim();
		} catch {
			return "unknown";
		}
	})();
	return {
		build: {
			sourcemap: true
		},
		plugins: [
			tailwindcss(),
			sveltekit(),
		],
		define: {
			'process.env.NODE_ENV': `"${mode}"`,
			'__APP_GIT_HASH__': JSON.stringify(git_hash),
			'__APP_NAME__': JSON.stringify(`radroots/${app_package.name ?? "app"}`),
			'__APP_VERSION__': JSON.stringify(app_package.version ?? "0.0.0"),
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
