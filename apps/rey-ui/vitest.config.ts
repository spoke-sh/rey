import stylex from "@stylexjs/unplugin";
import { defineConfig } from "vitest/config";

const fromUi = (path: string) => new URL(path, import.meta.url).pathname;

export default defineConfig({
  plugins: [stylex.rollup({ devMode: "full", useCSSLayers: true })],
  resolve: {
    alias: [
      {
        find: "@hifi/kinetic/grammar",
        replacement: fromUi("./vendor/hifi/packages/kinetic/src/grammar.ts"),
      },
      {
        find: "@hifi/core",
        replacement: fromUi("./vendor/hifi/packages/core/src/index.ts"),
      },
      {
        find: "@hifi/kinetic",
        replacement: fromUi("./vendor/hifi/packages/kinetic/src/index.ts"),
      },
    ],
  },
  test: {
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
