import { defineConfig } from "vitest/config";

export default defineConfig({
	test: {
		include: ["src/**/*.test.ts"],
		exclude: ["reference/**", "rea-main/**", ".dev/**", "node_modules/**"],
		environment: "node"
	}
});
