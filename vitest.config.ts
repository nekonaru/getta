import { defineConfig } from "vitest/config";

export default defineConfig({
	test: {
		include: ["src/**/*.test.ts"],
		exclude: ["omniget-main/**", "rea-main/**", ".dev/**", "node_modules/**"],
		environment: "node"
	}
});
