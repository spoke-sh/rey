import stylex from "@stylexjs/unplugin";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [stylex.rollup({ devMode: "full", useCSSLayers: true })],
  test: {
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
