import adapter from "@sveltejs/adapter-static";
import { sveltekit } from "@sveltejs/kit/vite";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";

export default defineConfig({
	plugins: [
		sveltekit({
			preprocess: vitePreprocess(),
			adapter: adapter({
				pages: "build",
				assets: "build",
				fallback: "index.html",
				precompress: false,
				strict: false
			}),
			paths: {
				relative: true
			}
		})
	],
	clearScreen: false,
	server: {
		port: 1420,
		strictPort: true,
		watch: {
			ignored: ["**/src-tauri/**"]
		}
	},
	envPrefix: ["VITE_", "TAURI_ENV_"],
	build: {
		target: "es2022"
	}
});
